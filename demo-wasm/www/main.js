import init, {
    generate_timed,
    generate_3d_timed,
    StepSolver,
    StepSolver3D,
    get_terrain_color,
    get_dungeon_color,
    get_terrain_name,
    get_dungeon_name,
    get_voxel_color,
    get_voxel_name,
    get_voxel_num_states,
    get_num_states,
} from './pkg/wavfc_demo.js';

import { init3D, render3D, updateVoxel, setSliceLevel, dispose3D, isInitialized } from './renderer3d.js';

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

const VOXEL_TERRAIN_COLORS = [
    '#87CEEB', '#4CAF50', '#8B6914', '#808080', '#FFD700', '#1A1A1A',
];
const VOXEL_TERRAIN_NAMES = [
    'Air', 'Grass', 'Dirt', 'Stone', 'Ore', 'Bedrock',
];

const VOXEL_FOREST_COLORS = [
    '#87CEEB', '#4CAF50', '#8B6914', '#808080', '#FFD700', '#1A1A1A',
    '#6D4C41', '#2E7D32',
];
const VOXEL_FOREST_NAMES = [
    'Air', 'Grass', 'Dirt', 'Stone', 'Ore', 'Bedrock',
    'Trunk', 'Leaves',
];

const VOXEL_VILLAGE_COLORS = [
    '#87CEEB', '#4CAF50', '#8B6914', '#808080', '#FFD700', '#1A1A1A',
    '#6D4C41', '#2E7D32', '#795548', '#D4A574', '#B71C1C',
];
const VOXEL_VILLAGE_NAMES = [
    'Air', 'Grass', 'Dirt', 'Stone', 'Ore', 'Bedrock',
    'Trunk', 'Leaves', 'Wall', 'Floor', 'Roof',
];

// ---------------------------------------------------------------------------
// DOM references
// ---------------------------------------------------------------------------
const elMode          = document.getElementById('mode');
const elTileset       = document.getElementById('tileset');
const elTileset3D     = document.getElementById('tileset-3d');
const elWidth         = document.getElementById('width');
const elHeight        = document.getElementById('height');
const elDepth         = document.getElementById('depth');
const elSeed          = document.getElementById('seed');
const elRandomSeed    = document.getElementById('random-seed');
const elWrapping      = document.getElementById('wrapping');
const elPropagator    = document.getElementById('propagator');
const elHeuristic     = document.getElementById('heuristic');
const elBacktrack     = document.getElementById('backtrack');
const elBacktrackParam = document.getElementById('backtrack-param');
const elAutoSpeed     = document.getElementById('auto-speed');
const elSpeedLabel    = document.getElementById('speed-label');
const elLayerSlice    = document.getElementById('layer-slice');
const elLayerLabel    = document.getElementById('layer-label');
const elGenerate      = document.getElementById('generate');
const elStepStart     = document.getElementById('step-start');
const elStepNext      = document.getElementById('step-next');
const elStepAuto      = document.getElementById('step-auto');
const elMessage       = document.getElementById('message');
const elStats         = document.getElementById('stats');
const elLegend        = document.getElementById('legend');
const elHowItWorks    = document.getElementById('how-it-works-content');
const canvas          = document.getElementById('canvas');
const ctx             = canvas.getContext('2d');
const threeContainer  = document.getElementById('three-container');

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------
let currentMode  = '2d';
let stepSolver   = null;
let stepSolver3D = null;
let stepStates   = null;
let stepWidth    = 0;
let stepHeight   = 0;
let stepDepth    = 0;
let stepTileset  = 'terrain';
let stepCount    = 0;
let autoPlaying  = false;
let autoRafId    = null;
let full3DStates = null;
let currentDepth = 0;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function getColor(state, tileset) {
    if (state === 255) return '#333333';
    if (tileset === 'dungeon') return DUNGEON_COLORS[state] || '#333333';
    if (tileset === 'terrain_3d') return VOXEL_TERRAIN_COLORS[state] || '#333333';
    if (tileset === 'forest') return VOXEL_FOREST_COLORS[state] || '#333333';
    if (tileset === 'village') return VOXEL_VILLAGE_COLORS[state] || '#333333';
    return TERRAIN_COLORS[state] || '#333333';
}

function getColors(tileset) {
    if (tileset === 'dungeon') return DUNGEON_COLORS;
    if (tileset === 'terrain_3d') return VOXEL_TERRAIN_COLORS;
    if (tileset === 'forest') return VOXEL_FOREST_COLORS;
    if (tileset === 'village') return VOXEL_VILLAGE_COLORS;
    return TERRAIN_COLORS;
}

function getNames(tileset) {
    if (tileset === 'dungeon') return DUNGEON_NAMES;
    if (tileset === 'terrain_3d') return VOXEL_TERRAIN_NAMES;
    if (tileset === 'forest') return VOXEL_FOREST_NAMES;
    if (tileset === 'village') return VOXEL_VILLAGE_NAMES;
    return TERRAIN_NAMES;
}

function readParams() {
    const mode = elMode.value;
    const maxSize = mode === '3d' ? 32 : 128;
    return {
        mode,
        width:      Math.max(4, Math.min(maxSize, parseInt(elWidth.value, 10) || 32)),
        height:     Math.max(4, Math.min(maxSize, parseInt(elHeight.value, 10) || 32)),
        depth:      Math.max(4, Math.min(32, parseInt(elDepth.value, 10) || 12)),
        seed:       BigInt(parseInt(elSeed.value, 10) || 0),
        tileset:    elTileset.value,
        tileset3d:  elTileset3D.value,
        wrapping:   elWrapping.checked,
        propagator: elPropagator.value,
        heuristic:  elHeuristic.value,
        backtrack_mode:  elBacktrack.value,
        backtrack_param: parseInt(elBacktrackParam.value, 10) || 10,
    };
}

// ---------------------------------------------------------------------------
// Mode switching
// ---------------------------------------------------------------------------
function updateModeVisibility() {
    currentMode = elMode.value;
    const is3D = currentMode === '3d';

    document.querySelectorAll('.mode-2d-only').forEach(el => {
        el.classList.toggle('hidden', is3D);
    });
    document.querySelectorAll('.mode-3d-only').forEach(el => {
        el.classList.toggle('hidden', !is3D);
    });

    if (is3D) {
        if (!isInitialized()) {
            init3D(threeContainer);
        }
    } else {
        dispose3D();
    }

    // Update legend and how-it-works for current tileset
    const tileset = is3D ? elTileset3D.value : elTileset.value;
    buildLegend(tileset);
    buildHowItWorks(tileset);
}

// ---------------------------------------------------------------------------
// Canvas rendering (2D)
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
    const colors = getColors(tileset);
    const names  = getNames(tileset);

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
// How It Works
// ---------------------------------------------------------------------------
const ADJACENCY_2D = {
    terrain: {
        // Gradient: each state adjacent to itself and neighbors within 1
        check: (a, b) => Math.abs(a - b) <= 1,
        note: 'Each tile can neighbor itself and tiles within 1 step in the gradient.',
    },
    dungeon: {
        check: (a, b) => {
            const lo = Math.min(a, b), hi = Math.max(a, b);
            return [[0,0],[0,1],[0,3],[1,1],[1,2],[1,3],[1,4],[2,2],[2,3],[4,4]]
                .some(([x, y]) => x === lo && y === hi);
        },
        note: 'Specific pairs: Wall-Floor, Wall-Door, Floor-Corridor, Floor-Door, Floor-Pillar, etc.',
    },
};

const ADJACENCY_3D_HORIZ = {
    terrain_3d: (a, b) => {
        const lo = Math.min(a, b), hi = Math.max(a, b);
        return [[0,0],[1,1],[2,2],[3,3],[4,4],[5,5],
                [0,1],[1,2],[2,3],[3,4],[3,5],[0,2],[0,3]]
            .some(([x, y]) => x === lo && y === hi);
    },
    forest: (a, b) => {
        const lo = Math.min(a, b), hi = Math.max(a, b);
        return [[0,0],[1,1],[2,2],[3,3],[4,4],[5,5],
                [0,1],[1,2],[2,3],[3,4],[3,5],[0,2],[0,3],
                [0,6],[0,7],[7,7],[6,7],[1,6],[6,6]]
            .some(([x, y]) => x === lo && y === hi);
    },
    village: (a, b) => {
        const lo = Math.min(a, b), hi = Math.max(a, b);
        return [[0,0],[1,1],[2,2],[3,3],[4,4],[5,5],
                [0,1],[1,2],[2,3],[3,4],[3,5],[0,2],[0,3],
                [0,6],[0,7],[7,7],[6,7],[1,6],[6,6],
                [8,8],[8,9],[0,8],[1,8],[9,9],[10,10],[0,10]]
            .some(([x, y]) => x === lo && y === hi);
    },
};

const ADJACENCY_3D_VERT = {
    terrain_3d: (lower, upper) => {
        return [[5,5],[5,3],[3,3],[3,2],[3,4],[3,0],
                [4,3],[4,4],[2,2],[2,1],[2,0],[1,0],[0,0]]
            .some(([lo, up]) => lo === lower && up === upper);
    },
    forest: (lower, upper) => {
        return [[5,5],[5,3],[3,3],[3,2],[3,4],[3,0],
                [4,3],[4,4],[2,2],[2,1],[2,0],[1,0],[0,0],
                [1,6],[6,6],[6,7],[7,7],[7,0]]
            .some(([lo, up]) => lo === lower && up === upper);
    },
    village: (lower, upper) => {
        return [[5,5],[5,3],[3,3],[3,2],[3,4],[3,0],
                [4,3],[4,4],[2,2],[2,1],[2,0],[1,0],[0,0],
                [1,6],[6,6],[6,7],[7,7],[7,0],
                [1,8],[1,9],[9,8],[9,0],[8,8],[8,10],[8,0],[10,0],[9,9]]
            .some(([lo, up]) => lo === lower && up === upper);
    },
};

function buildHowItWorks(tileset) {
    const colors = getColors(tileset);
    const names  = getNames(tileset);
    const n = colors.length;
    const is3D = currentMode === '3d';

    let html = '<p>Wave Function Collapse works like a jigsaw puzzle \u2014 each tile has rules about ' +
        'which tiles can be its neighbors. The algorithm places tiles one at a time, always ' +
        'respecting these rules, until the whole grid is filled.</p>';

    if (is3D) {
        html += '<h4>Horizontal Adjacency</h4>';
        html += buildMatrix(n, colors, names, (a, b) => {
            const fn = ADJACENCY_3D_HORIZ[tileset];
            return fn ? fn(a, b) : false;
        });
        html += '<h4>Vertical Adjacency (lower \u2192 upper)</h4>';
        html += buildMatrix(n, colors, names, (a, b) => {
            const fn = ADJACENCY_3D_VERT[tileset];
            return fn ? fn(a, b) : false;
        });
        html += '<p class="matrix-note">Vertical rules are directional: rows = lower block, columns = upper block.</p>';
    } else {
        const adj = ADJACENCY_2D[tileset];
        if (adj) {
            html += '<h4>Adjacency Matrix</h4>';
            html += buildMatrix(n, colors, names, adj.check);
            html += '<p class="matrix-note">' + adj.note + '</p>';
        }
    }

    elHowItWorks.innerHTML = html;
}

function buildMatrix(n, colors, names, checkFn) {
    let html = '<table class="adjacency-matrix"><thead><tr><th></th>';
    for (let i = 0; i < n; i++) {
        html += '<th><span class="matrix-swatch" style="background:' + colors[i] + '"></span>' +
            names[i].split(' ').pop() + '</th>';
    }
    html += '</tr></thead><tbody>';
    for (let a = 0; a < n; a++) {
        html += '<tr><th><span class="matrix-swatch" style="background:' + colors[a] + '"></span>' +
            names[a] + '</th>';
        for (let b = 0; b < n; b++) {
            const ok = checkFn(a, b);
            html += '<td class="' + (ok ? 'compat' : 'incompat') + '">' + (ok ? '\u2713' : '') + '</td>';
        }
        html += '</tr>';
    }
    html += '</tbody></table>';
    return html;
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
    if (val < 10) {
        const framesPerStep = 11 - val;
        elSpeedLabel.textContent = '1 step/' + framesPerStep + ' frames';
    } else if (val === 10) {
        elSpeedLabel.textContent = '1 step/frame';
    } else {
        elSpeedLabel.textContent = (val - 9) + ' steps/frame';
    }
}

// ---------------------------------------------------------------------------
// Layer slice
// ---------------------------------------------------------------------------
function updateLayerSlice() {
    const val = parseInt(elLayerSlice.value, 10);
    const max = parseInt(elLayerSlice.max, 10);
    if (val >= max) {
        elLayerLabel.textContent = 'All';
    } else {
        elLayerLabel.textContent = 'Z \u2264 ' + val;
    }
    if (isInitialized()) {
        setSliceLevel(val);
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
    if (stepSolver3D) {
        stepSolver3D.free();
        stepSolver3D = null;
    }
    stepStates  = null;
    full3DStates = null;
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

    if (p.mode === '3d') {
        doGenerate3D(p);
    } else {
        doGenerate2D(p);
    }
}

function doGenerate2D(p) {
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
            'Grid':    result.width + ' \u00d7 ' + result.height,
            'Tileset': p.tileset.charAt(0).toUpperCase() + p.tileset.slice(1),
            'Seed':    p.seed,
            'Time':    result.elapsed_ms.toFixed(2) + ' ms',
        });

        showMessage('Generation complete', 'success');
    } catch (err) {
        showMessage('Generation failed: ' + err, 'error');
    }
}

function doGenerate3D(p) {
    if (!isInitialized()) {
        init3D(threeContainer);
    }

    try {
        const json = generate_3d_timed(
            p.width, p.height, p.depth, p.seed, p.tileset3d,
            p.wrapping, p.propagator, p.heuristic,
            p.backtrack_mode, p.backtrack_param,
        );
        const result = JSON.parse(json);

        const states = new Uint8Array(result.states);
        full3DStates = states;
        currentDepth = result.depth;

        const colorFn = (state) => getColor(state, p.tileset3d);
        render3D(states, result.width, result.height, result.depth, p.tileset3d, colorFn);

        // Setup layer slice
        elLayerSlice.max = result.depth - 1;
        elLayerSlice.value = result.depth - 1;
        elLayerLabel.textContent = 'All';

        showStats({
            'Grid':    result.width + ' \u00d7 ' + result.height + ' \u00d7 ' + result.depth,
            'Tileset': p.tileset3d.charAt(0).toUpperCase() + p.tileset3d.slice(1).replace('_3d', ''),
            'Seed':    p.seed,
            'Time':    result.elapsed_ms.toFixed(2) + ' ms',
        });

        showMessage('3D generation complete', 'success');
    } catch (err) {
        showMessage('3D generation failed: ' + err, 'error');
    }
}

// ---------------------------------------------------------------------------
// Step-through mode
// ---------------------------------------------------------------------------
function startStepThrough() {
    resetStepState();
    hideMessage();

    const p = readParams();

    if (p.mode === '3d') {
        startStepThrough3D(p);
    } else {
        startStepThrough2D(p);
    }
}

function startStepThrough2D(p) {
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
        'Grid':    stepWidth + ' \u00d7 ' + stepHeight,
        'Tileset': stepTileset.charAt(0).toUpperCase() + stepTileset.slice(1),
        'Steps':   '0 / ' + (stepWidth * stepHeight),
    });

    elStepNext.disabled = false;
    elStepAuto.disabled = false;
    showMessage('Step-through started. Press Next Step or Auto Play.', 'info');
}

function startStepThrough3D(p) {
    stepWidth   = p.width;
    stepHeight  = p.height;
    stepDepth   = p.depth;
    stepTileset = p.tileset3d;
    stepCount   = 0;

    if (!isInitialized()) {
        init3D(threeContainer);
    }

    try {
        stepSolver3D = new StepSolver3D(
            p.width, p.height, p.depth, p.seed, p.tileset3d,
            p.wrapping, p.propagator, p.heuristic,
        );
    } catch (err) {
        showMessage('Failed to create 3D solver: ' + err, 'error');
        return;
    }

    // Initialize
    stepStates = new Uint8Array(p.width * p.height * p.depth);
    stepStates.fill(255);

    // Clear the 3D scene
    const colorFn = (state) => getColor(state, stepTileset);
    render3D(stepStates, stepWidth, stepHeight, stepDepth, stepTileset, colorFn);

    // Setup layer slice
    elLayerSlice.max = stepDepth - 1;
    elLayerSlice.value = stepDepth - 1;
    elLayerLabel.textContent = 'All';

    const total = stepWidth * stepHeight * stepDepth;
    showStats({
        'Grid':    stepWidth + ' \u00d7 ' + stepHeight + ' \u00d7 ' + stepDepth,
        'Tileset': stepTileset.charAt(0).toUpperCase() + stepTileset.slice(1).replace('_3d', ''),
        'Steps':   '0 / ' + total,
    });

    elStepNext.disabled = false;
    elStepAuto.disabled = false;
    showMessage('3D step-through started. Press Next Step or Auto Play.', 'info');
}

function doStep() {
    if (currentMode === '3d') return doStep3D();
    return doStep2D();
}

function doStep2D() {
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
            'Grid':    stepWidth + ' \u00d7 ' + stepHeight,
            'Tileset': stepTileset.charAt(0).toUpperCase() + stepTileset.slice(1),
            'Steps':   stepCount + ' / ' + (stepWidth * stepHeight),
        });
        return true;
    }

    if (result.type === 'complete') {
        stopAutoPlay();
        showStats({
            'Grid':    stepWidth + ' \u00d7 ' + stepHeight,
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

function doStep3D() {
    if (!stepSolver3D) return false;

    let resultJson;
    try {
        resultJson = stepSolver3D.step();
    } catch (err) {
        stopAutoPlay();
        showMessage('3D solver error: ' + err, 'error');
        return false;
    }

    const result = JSON.parse(resultJson);
    const total = stepWidth * stepHeight * stepDepth;

    if (result.type === 'collapsed') {
        stepCount++;
        stepStates[result.cell] = result.state;

        const colorFn = (state) => getColor(state, stepTileset);
        updateVoxel(result.cell, result.state, stepWidth, stepHeight, stepDepth, stepTileset, colorFn);

        showStats({
            'Grid':    stepWidth + ' \u00d7 ' + stepHeight + ' \u00d7 ' + stepDepth,
            'Tileset': stepTileset.charAt(0).toUpperCase() + stepTileset.slice(1).replace('_3d', ''),
            'Steps':   stepCount + ' / ' + total,
        });
        return true;
    }

    if (result.type === 'complete') {
        stopAutoPlay();
        showStats({
            'Grid':    stepWidth + ' \u00d7 ' + stepHeight + ' \u00d7 ' + stepDepth,
            'Tileset': stepTileset.charAt(0).toUpperCase() + stepTileset.slice(1).replace('_3d', ''),
            'Steps':   stepCount + ' / ' + total,
            'Status':  'Complete',
        });
        showMessage('3D generation complete!', 'success');
        elStepNext.disabled = true;
        elStepAuto.disabled = true;
        return false;
    }

    if (result.type === 'contradiction') {
        stopAutoPlay();
        showMessage('3D contradiction at cell ' + result.cell, 'error');
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

    let frameCounter = 0;

    function tick() {
        if (!autoPlaying) return;

        const val = parseInt(elAutoSpeed.value, 10) || 10;
        let keepGoing = true;

        if (val < 10) {
            // Slow mode: 1 step every N frames
            const framesPerStep = 11 - val;
            frameCounter++;
            if (frameCounter >= framesPerStep) {
                frameCounter = 0;
                keepGoing = doStep();
            }
        } else {
            // Fast mode: N steps per frame
            const stepsPerFrame = val <= 10 ? 1 : val - 9;
            for (let i = 0; i < stepsPerFrame && keepGoing; i++) {
                keepGoing = doStep();
            }
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
    if (stepSolver || stepSolver3D) {
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
elLayerSlice.addEventListener('input', updateLayerSlice);

elMode.addEventListener('change', () => {
    resetStepState();
    updateModeVisibility();
});

elTileset.addEventListener('change', () => {
    buildLegend(elTileset.value);
    buildHowItWorks(elTileset.value);
});

elTileset3D.addEventListener('change', () => {
    buildLegend(elTileset3D.value);
    buildHowItWorks(elTileset3D.value);
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
    updateModeVisibility();

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
