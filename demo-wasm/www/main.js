import init, {
    generate_timed,
    StepSolver,
    get_terrain_color,
    get_dungeon_color,
    get_terrain_name,
    get_dungeon_name,
    get_num_states,
} from '../pkg/wavfc_demo.js';

// ---------------------------------------------------------------------------
// Color / name lookup tables (hardcoded to match Rust side)
// ---------------------------------------------------------------------------
const TERRAIN_COLORS = [
    '#1a5276', '#2e86c1', '#f9e79f', '#27ae60',
    '#1e8449', '#af7ac5', '#7f8c8d',
];
const TERRAIN_NAMES = [
    'Deep Water', 'Water', 'Sand', 'Grass',
    'Forest', 'Hills', 'Mountain',
];

const DUNGEON_COLORS = [
    '#2c3e50', '#d4a574', '#c4a064', '#8b4513', '#95a5a6',
];
const DUNGEON_NAMES = [
    'Wall', 'Floor', 'Corridor', 'Door', 'Pillar',
];

// ---------------------------------------------------------------------------
// DOM references
// ---------------------------------------------------------------------------
const elTileset       = document.getElementById('tileset');
const elWidth         = document.getElementById('width');
const elHeight        = document.getElementById('height');
const elSeed          = document.getElementById('seed');
const elRandomSeed    = document.getElementById('random-seed');
const elWrapping      = document.getElementById('wrapping');
const elPropagator    = document.getElementById('propagator');
const elHeuristic     = document.getElementById('heuristic');
const elBacktrack     = document.getElementById('backtrack');
const elBacktrackParam = document.getElementById('backtrack-param');
const elAutoSpeed     = document.getElementById('auto-speed');
const elSpeedLabel    = document.getElementById('speed-label');
const elGenerate      = document.getElementById('generate');
const elStepStart     = document.getElementById('step-start');
const elStepNext      = document.getElementById('step-next');
const elStepAuto      = document.getElementById('step-auto');
const elMessage       = document.getElementById('message');
const elStats         = document.getElementById('stats');
const elLegend        = document.getElementById('legend');
const canvas          = document.getElementById('canvas');
const ctx             = canvas.getContext('2d');

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------
let stepSolver   = null;
let stepStates   = null;
let stepWidth    = 0;
let stepHeight   = 0;
let stepTileset  = 'terrain';
let stepCount    = 0;
let autoPlaying  = false;
let autoRafId    = null;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function getColor(state, tileset) {
    if (state === 255) return '#333333';
    if (tileset === 'dungeon') {
        return DUNGEON_COLORS[state] || '#333333';
    }
    return TERRAIN_COLORS[state] || '#333333';
}

function readParams() {
    return {
        width:      Math.max(4, Math.min(128, parseInt(elWidth.value, 10) || 32)),
        height:     Math.max(4, Math.min(128, parseInt(elHeight.value, 10) || 32)),
        seed:       parseInt(elSeed.value, 10) || 0,
        tileset:    elTileset.value,
        wrapping:   elWrapping.checked,
        propagator: elPropagator.value,
        heuristic:  elHeuristic.value,
        backtrack_mode:  elBacktrack.value,
        backtrack_param: parseInt(elBacktrackParam.value, 10) || 10,
    };
}

// ---------------------------------------------------------------------------
// Canvas rendering
// ---------------------------------------------------------------------------
function renderGrid(states, width, height, tileset) {
    const maxSize = 600;
    const cellSize = Math.max(2, Math.floor(maxSize / Math.max(width, height)));

    canvas.width  = width  * cellSize;
    canvas.height = height * cellSize;

    for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
            const state = states[y * width + x];
            ctx.fillStyle = getColor(state, tileset);
            ctx.fillRect(x * cellSize, y * cellSize, cellSize, cellSize);
        }
    }

    // Draw grid lines for cells large enough to see them
    if (cellSize >= 8) {
        ctx.strokeStyle = 'rgba(0,0,0,0.15)';
        ctx.lineWidth = 0.5;
        for (let x = 0; x <= width; x++) {
            ctx.beginPath();
            ctx.moveTo(x * cellSize, 0);
            ctx.lineTo(x * cellSize, height * cellSize);
            ctx.stroke();
        }
        for (let y = 0; y <= height; y++) {
            ctx.beginPath();
            ctx.moveTo(0, y * cellSize);
            ctx.lineTo(width * cellSize, y * cellSize);
            ctx.stroke();
        }
    }
}

function highlightCell(cellIndex, width, height, tileset) {
    const maxSize = 600;
    const cellSize = Math.max(2, Math.floor(maxSize / Math.max(width, height)));
    const x = cellIndex % width;
    const y = Math.floor(cellIndex / width);

    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 2;
    ctx.strokeRect(x * cellSize + 1, y * cellSize + 1, cellSize - 2, cellSize - 2);

    // Fade the highlight after a brief moment
    setTimeout(() => {
        if (stepStates) {
            // Redraw just this cell without the highlight
            const state = stepStates[cellIndex];
            ctx.fillStyle = getColor(state, tileset);
            ctx.fillRect(x * cellSize, y * cellSize, cellSize, cellSize);

            // Restore grid line if needed
            if (cellSize >= 8) {
                ctx.strokeStyle = 'rgba(0,0,0,0.15)';
                ctx.lineWidth = 0.5;
                ctx.strokeRect(x * cellSize, y * cellSize, cellSize, cellSize);
            }
        }
    }, 120);
}

// ---------------------------------------------------------------------------
// Legend
// ---------------------------------------------------------------------------
function buildLegend(tileset) {
    const colors = tileset === 'dungeon' ? DUNGEON_COLORS : TERRAIN_COLORS;
    const names  = tileset === 'dungeon' ? DUNGEON_NAMES  : TERRAIN_NAMES;

    elLegend.innerHTML = '<h3>Legend</h3>';
    colors.forEach((color, i) => {
        const item = document.createElement('div');
        item.className = 'legend-item';
        item.innerHTML =
            '<span class="swatch" style="background:' + color + '"></span>' +
            names[i];
        elLegend.appendChild(item);
    });
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------
function showStats(data) {
    const rows = Object.entries(data).map(([label, value]) =>
        '<div class="stat-row">' +
        '<span class="stat-label">' + label + '</span>' +
        '<span class="stat-value">' + value + '</span>' +
        '</div>'
    );
    elStats.innerHTML = rows.join('');
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------
function showMessage(text, type) {
    type = type || 'info';
    elMessage.textContent = text;
    elMessage.className = 'message ' + type;
    elMessage.classList.remove('hidden');
}

function hideMessage() {
    elMessage.classList.add('hidden');
}

// ---------------------------------------------------------------------------
// Backtrack param visibility
// ---------------------------------------------------------------------------
function updateBacktrackParamVisibility() {
    if (elBacktrack.value === 'none') {
        elBacktrackParam.classList.add('hidden');
    } else {
        elBacktrackParam.classList.remove('hidden');
    }
}

// ---------------------------------------------------------------------------
// Speed label
// ---------------------------------------------------------------------------
function updateSpeedLabel() {
    const val = parseInt(elAutoSpeed.value, 10);
    if (val === 1) {
        elSpeedLabel.textContent = '1 step/frame';
    } else {
        elSpeedLabel.textContent = val + ' steps/frame';
    }
}

// ---------------------------------------------------------------------------
// Stop any active step-through / auto-play
// ---------------------------------------------------------------------------
function resetStepState() {
    stopAutoPlay();
    if (stepSolver) {
        stepSolver.free();
        stepSolver = null;
    }
    stepStates  = null;
    stepCount   = 0;
    elStepNext.disabled = true;
    elStepAuto.disabled = true;
    elStepAuto.textContent = 'Auto Play';
    elStepAuto.classList.remove('active');
}

// ---------------------------------------------------------------------------
// Full generation
// ---------------------------------------------------------------------------
function doGenerate() {
    resetStepState();
    hideMessage();

    const p = readParams();

    try {
        const json = generate_timed(
            p.width, p.height, p.seed, p.tileset,
            p.wrapping, p.propagator, p.heuristic,
            p.backtrack_mode, p.backtrack_param,
        );
        const result = JSON.parse(json);

        const states = new Uint8Array(result.states);
        renderGrid(states, result.width, result.height, p.tileset);

        showStats({
            'Grid':    result.width + ' x ' + result.height,
            'Tileset': p.tileset.charAt(0).toUpperCase() + p.tileset.slice(1),
            'Seed':    p.seed,
            'Time':    result.elapsed_ms.toFixed(2) + ' ms',
        });

        showMessage('Generation complete', 'success');
    } catch (err) {
        showMessage('Generation failed: ' + err, 'error');
    }
}

// ---------------------------------------------------------------------------
// Step-through mode
// ---------------------------------------------------------------------------
function startStepThrough() {
    resetStepState();
    hideMessage();

    const p = readParams();
    stepWidth   = p.width;
    stepHeight  = p.height;
    stepTileset = p.tileset;
    stepCount   = 0;

    try {
        stepSolver = new StepSolver(
            p.width, p.height, p.seed, p.tileset,
            p.wrapping, p.propagator, p.heuristic,
        );
    } catch (err) {
        showMessage('Failed to create solver: ' + err, 'error');
        return;
    }

    // Initialize all cells as uncollapsed
    stepStates = new Uint8Array(p.width * p.height);
    stepStates.fill(255);

    renderGrid(stepStates, stepWidth, stepHeight, stepTileset);
    showStats({
        'Grid':    stepWidth + ' x ' + stepHeight,
        'Tileset': stepTileset.charAt(0).toUpperCase() + stepTileset.slice(1),
        'Steps':   '0 / ' + (stepWidth * stepHeight),
    });

    elStepNext.disabled = false;
    elStepAuto.disabled = false;
    showMessage('Step-through started. Press Next Step or Auto Play.', 'info');
}

function doStep() {
    if (!stepSolver) return false;

    let resultJson;
    try {
        resultJson = stepSolver.step();
    } catch (err) {
        stopAutoPlay();
        showMessage('Solver error: ' + err, 'error');
        return false;
    }

    const result = JSON.parse(resultJson);

    if (result.type === 'collapsed') {
        stepCount++;
        stepStates[result.cell] = result.state;
        renderGrid(stepStates, stepWidth, stepHeight, stepTileset);

        if (!autoPlaying) {
            highlightCell(result.cell, stepWidth, stepHeight, stepTileset);
        }

        showStats({
            'Grid':    stepWidth + ' x ' + stepHeight,
            'Tileset': stepTileset.charAt(0).toUpperCase() + stepTileset.slice(1),
            'Steps':   stepCount + ' / ' + (stepWidth * stepHeight),
        });
        return true;
    }

    if (result.type === 'complete') {
        stopAutoPlay();
        showStats({
            'Grid':    stepWidth + ' x ' + stepHeight,
            'Tileset': stepTileset.charAt(0).toUpperCase() + stepTileset.slice(1),
            'Steps':   stepCount + ' / ' + (stepWidth * stepHeight),
            'Status':  'Complete',
        });
        showMessage('Generation complete!', 'success');
        elStepNext.disabled = true;
        elStepAuto.disabled = true;
        return false;
    }

    if (result.type === 'contradiction') {
        stopAutoPlay();
        showMessage('Contradiction at cell ' + result.cell, 'error');
        elStepNext.disabled = true;
        elStepAuto.disabled = true;
        return false;
    }

    return false;
}

// ---------------------------------------------------------------------------
// Auto-play
// ---------------------------------------------------------------------------
function startAutoPlay() {
    autoPlaying = true;
    elStepAuto.textContent = 'Stop';
    elStepAuto.classList.add('active');
    elStepNext.disabled = true;

    function tick() {
        if (!autoPlaying) return;

        const stepsPerFrame = parseInt(elAutoSpeed.value, 10) || 1;
        let keepGoing = true;
        for (let i = 0; i < stepsPerFrame && keepGoing; i++) {
            keepGoing = doStep();
        }

        if (keepGoing && autoPlaying) {
            autoRafId = requestAnimationFrame(tick);
        } else {
            stopAutoPlay();
        }
    }

    autoRafId = requestAnimationFrame(tick);
}

function stopAutoPlay() {
    autoPlaying = false;
    if (autoRafId !== null) {
        cancelAnimationFrame(autoRafId);
        autoRafId = null;
    }
    elStepAuto.textContent = 'Auto Play';
    elStepAuto.classList.remove('active');
    if (stepSolver) {
        elStepNext.disabled = false;
    }
}

function toggleAutoPlay() {
    if (autoPlaying) {
        stopAutoPlay();
    } else {
        startAutoPlay();
    }
}

// ---------------------------------------------------------------------------
// Event listeners
// ---------------------------------------------------------------------------
elGenerate.addEventListener('click', doGenerate);
elStepStart.addEventListener('click', startStepThrough);
elStepNext.addEventListener('click', () => { doStep(); });
elStepAuto.addEventListener('click', toggleAutoPlay);

elRandomSeed.addEventListener('click', () => {
    elSeed.value = Math.floor(Math.random() * 2147483647);
});

elBacktrack.addEventListener('change', updateBacktrackParamVisibility);
elAutoSpeed.addEventListener('input', updateSpeedLabel);

elTileset.addEventListener('change', () => {
    buildLegend(elTileset.value);
});

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------
async function main() {
    try {
        await init();
    } catch (err) {
        showMessage('Failed to load WASM module: ' + err, 'error');
        return;
    }

    // Initial UI state
    updateBacktrackParamVisibility();
    updateSpeedLabel();
    buildLegend(elTileset.value);

    // Draw an empty placeholder canvas
    const p = readParams();
    const maxSize = 600;
    const cellSize = Math.max(2, Math.floor(maxSize / Math.max(p.width, p.height)));
    canvas.width  = p.width  * cellSize;
    canvas.height = p.height * cellSize;
    ctx.fillStyle = '#222';
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // Draw centered placeholder text
    ctx.fillStyle = '#555';
    ctx.font = '16px sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('Click Generate to start', canvas.width / 2, canvas.height / 2);

    showMessage('WASM module loaded. Ready to generate.', 'info');
}

main();
