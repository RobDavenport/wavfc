import * as THREE from 'three';
import { PointerLockControls } from 'three/addons/controls/PointerLockControls.js';

let scene, camera, renderer, controls;
let container = null;
let animationId = null;
let meshes = [];
let sliceLevel = Infinity;

// Movement state
const moveState = { forward: false, backward: false, left: false, right: false, up: false, down: false };
const velocity = new THREE.Vector3();
const direction = new THREE.Vector3();
const MOVE_SPEED = 20;
let prevTime = performance.now();

// Touch state
let touchLook = null;
let touchMove = null;
const TOUCH_LOOK_SENSITIVITY = 0.004;
const TOUCH_MOVE_SPEED = 15;

// Overlay elements
let overlayEl = null;
let hintsEl = null;
let hintsTimeout = null;

export function init3D(containerEl) {
    container = containerEl;

    scene = new THREE.Scene();
    scene.background = new THREE.Color(0x1a1a2e);
    scene.fog = new THREE.Fog(0x1a1a2e, 40, 120);

    camera = new THREE.PerspectiveCamera(70, container.clientWidth / container.clientHeight, 0.1, 200);

    renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(window.devicePixelRatio);
    renderer.setSize(container.clientWidth, container.clientHeight);
    container.appendChild(renderer.domElement);

    // Lighting
    const ambient = new THREE.AmbientLight(0xffffff, 0.6);
    scene.add(ambient);
    const directional = new THREE.DirectionalLight(0xffffff, 0.8);
    directional.position.set(1, 2, 1.5).normalize();
    scene.add(directional);

    // Pointer lock controls
    controls = new PointerLockControls(camera, renderer.domElement);

    // Overlay: "Click/tap to explore"
    overlayEl = document.createElement('div');
    overlayEl.className = 'three-overlay';
    overlayEl.innerHTML = '<div class="three-overlay-text">Click to explore<br><small>WASD to move, Mouse to look, Space/Shift up/down<br>Touch: left side = move, right side = look</small></div>';
    container.appendChild(overlayEl);

    overlayEl.addEventListener('click', () => {
        // Try pointer lock for desktop; on mobile it may fail silently
        controls.lock();
        // On touch devices, dismiss overlay directly since pointer lock may not work
        if ('ontouchstart' in window) {
            overlayEl.style.display = 'none';
        }
    });

    controls.addEventListener('lock', () => {
        overlayEl.style.display = 'none';
        showHints();
    });

    controls.addEventListener('unlock', () => {
        overlayEl.style.display = '';
        hideHints();
    });

    // Hints overlay
    hintsEl = document.createElement('div');
    hintsEl.className = 'three-hints three-hints-hidden';
    hintsEl.textContent = 'WASD: Move | Mouse: Look | Space: Up | Shift: Down | ESC: Release';
    container.appendChild(hintsEl);

    // Keyboard
    document.addEventListener('keydown', onKeyDown);
    document.addEventListener('keyup', onKeyUp);

    // Touch
    renderer.domElement.addEventListener('touchstart', onTouchStart, { passive: false });
    renderer.domElement.addEventListener('touchmove', onTouchMove, { passive: false });
    renderer.domElement.addEventListener('touchend', onTouchEnd, { passive: false });

    // Resize
    window.addEventListener('resize', onResize);

    // Start render loop
    prevTime = performance.now();
    animate();
}

function showHints() {
    if (hintsEl) {
        hintsEl.classList.remove('three-hints-hidden');
        clearTimeout(hintsTimeout);
        hintsTimeout = setTimeout(() => {
            if (hintsEl) hintsEl.classList.add('three-hints-hidden');
        }, 4000);
    }
}

function hideHints() {
    if (hintsEl) hintsEl.classList.add('three-hints-hidden');
    clearTimeout(hintsTimeout);
}

function onKeyDown(e) {
    if (!controls || !controls.isLocked) return;
    switch (e.code) {
        case 'KeyW': case 'ArrowUp':    moveState.forward = true; break;
        case 'KeyS': case 'ArrowDown':  moveState.backward = true; break;
        case 'KeyA': case 'ArrowLeft':  moveState.left = true; break;
        case 'KeyD': case 'ArrowRight': moveState.right = true; break;
        case 'Space':    moveState.up = true; e.preventDefault(); break;
        case 'ShiftLeft': case 'ShiftRight': moveState.down = true; break;
    }
}

function onKeyUp(e) {
    switch (e.code) {
        case 'KeyW': case 'ArrowUp':    moveState.forward = false; break;
        case 'KeyS': case 'ArrowDown':  moveState.backward = false; break;
        case 'KeyA': case 'ArrowLeft':  moveState.left = false; break;
        case 'KeyD': case 'ArrowRight': moveState.right = false; break;
        case 'Space':    moveState.up = false; break;
        case 'ShiftLeft': case 'ShiftRight': moveState.down = false; break;
    }
}

function onTouchStart(e) {
    e.preventDefault();
    const rect = renderer.domElement.getBoundingClientRect();
    const midX = rect.width / 2;
    for (const touch of e.changedTouches) {
        const localX = touch.clientX - rect.left;
        if (localX < midX) {
            touchMove = { id: touch.identifier, startX: touch.clientX, startY: touch.clientY };
        } else {
            touchLook = { id: touch.identifier, lastX: touch.clientX, lastY: touch.clientY };
        }
    }
}

function onTouchMove(e) {
    e.preventDefault();
    for (const touch of e.changedTouches) {
        if (touchLook && touch.identifier === touchLook.id) {
            const dx = touch.clientX - touchLook.lastX;
            const dy = touch.clientY - touchLook.lastY;
            // Manually rotate camera
            camera.rotation.order = 'YXZ';
            camera.rotation.y -= dx * TOUCH_LOOK_SENSITIVITY;
            camera.rotation.x -= dy * TOUCH_LOOK_SENSITIVITY;
            camera.rotation.x = Math.max(-Math.PI / 2, Math.min(Math.PI / 2, camera.rotation.x));
            touchLook.lastX = touch.clientX;
            touchLook.lastY = touch.clientY;
        }
        if (touchMove && touch.identifier === touchMove.id) {
            // Movement handled in animate via displacement from start
            touchMove.currentX = touch.clientX;
            touchMove.currentY = touch.clientY;
        }
    }
}

function onTouchEnd(e) {
    for (const touch of e.changedTouches) {
        if (touchLook && touch.identifier === touchLook.id) touchLook = null;
        if (touchMove && touch.identifier === touchMove.id) touchMove = null;
    }
}

function onResize() {
    if (!container || !camera || !renderer) return;
    camera.aspect = container.clientWidth / container.clientHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(container.clientWidth, container.clientHeight);
}

function animate() {
    animationId = requestAnimationFrame(animate);

    const time = performance.now();
    const delta = (time - prevTime) / 1000;
    prevTime = time;

    // FPS movement
    if (controls && controls.isLocked) {
        velocity.set(0, 0, 0);
        direction.z = Number(moveState.forward) - Number(moveState.backward);
        direction.x = Number(moveState.right) - Number(moveState.left);
        direction.normalize();

        velocity.z -= direction.z * MOVE_SPEED * delta;
        velocity.x -= direction.x * MOVE_SPEED * delta;

        controls.moveRight(-velocity.x);
        controls.moveForward(-velocity.z);

        if (moveState.up) camera.position.y += MOVE_SPEED * delta;
        if (moveState.down) camera.position.y -= MOVE_SPEED * delta;
    }

    // Touch movement (no pointer lock needed)
    if (touchMove && touchMove.currentX !== undefined) {
        const dx = (touchMove.currentX - touchMove.startX) / 80;
        const dy = (touchMove.currentY - touchMove.startY) / 80;
        const forward = new THREE.Vector3();
        camera.getWorldDirection(forward);
        forward.y = 0;
        forward.normalize();
        const right = new THREE.Vector3().crossVectors(forward, new THREE.Vector3(0, 1, 0));
        camera.position.addScaledVector(forward, -dy * TOUCH_MOVE_SPEED * delta);
        camera.position.addScaledVector(right, dx * TOUCH_MOVE_SPEED * delta);
    }

    renderer.render(scene, camera);
}

export function render3D(states, width, height, depth, tileset, colorFn) {
    clearMeshes();
    sliceLevel = Infinity;

    // Group voxels by state
    const stateGroups = new Map();
    const layerSize = width * height;

    for (let z = 0; z < depth; z++) {
        for (let y = 0; y < height; y++) {
            for (let x = 0; x < width; x++) {
                const idx = z * layerSize + y * width + x;
                const state = states[idx];
                if (state === 0 || state === 255) continue; // skip air and uncollapsed
                if (!stateGroups.has(state)) stateGroups.set(state, []);
                stateGroups.get(state).push({ x, y, z });
            }
        }
    }

    const geometry = new THREE.BoxGeometry(1, 1, 1);

    for (const [state, positions] of stateGroups) {
        const color = colorFn(state);
        const material = new THREE.MeshLambertMaterial({ color });
        const mesh = new THREE.InstancedMesh(geometry, material, positions.length);

        const matrix = new THREE.Matrix4();
        positions.forEach((pos, i) => {
            matrix.setPosition(pos.x, pos.z, pos.y); // z in data -> y in Three.js (up)
            mesh.setMatrixAt(i, matrix);
        });
        mesh.instanceMatrix.needsUpdate = true;
        mesh.userData = { state, positions };
        scene.add(mesh);
        meshes.push(mesh);
    }

    // Position camera
    const cx = width / 2;
    const cy = depth * 0.6;
    const cz = height / 2;
    camera.position.set(cx + width * 0.8, cy, cz + height * 0.8);
    camera.lookAt(cx, depth * 0.3, cz);
}

export function updateVoxel(cell, state, width, height, depth, tileset, colorFn) {
    if (state === 0 || state === 255) return; // skip air
    const layerSize = width * height;
    const z = Math.floor(cell / layerSize);
    const rem = cell % layerSize;
    const y = Math.floor(rem / width);
    const x = rem % width;

    if (z > sliceLevel) return;

    // Find or create InstancedMesh for this state
    let targetMesh = meshes.find(m => m.userData.state === state);
    if (!targetMesh) {
        const geometry = new THREE.BoxGeometry(1, 1, 1);
        const color = colorFn(state);
        const material = new THREE.MeshLambertMaterial({ color });
        // Start with capacity for 256 instances, will grow if needed
        targetMesh = new THREE.InstancedMesh(geometry, material, 32768);
        targetMesh.count = 0;
        targetMesh.userData = { state, positions: [] };
        scene.add(targetMesh);
        meshes.push(targetMesh);
    }

    const matrix = new THREE.Matrix4();
    matrix.setPosition(x, z, y);
    const idx = targetMesh.count;
    targetMesh.setMatrixAt(idx, matrix);
    targetMesh.count = idx + 1;
    targetMesh.instanceMatrix.needsUpdate = true;
    targetMesh.userData.positions.push({ x, y, z });
}

export function setSliceLevel(level) {
    sliceLevel = level;
    // Rebuild visibility by hiding instances above the level
    for (const mesh of meshes) {
        const positions = mesh.userData.positions;
        const matrix = new THREE.Matrix4();
        let count = 0;
        for (const pos of positions) {
            if (pos.z <= level) {
                matrix.setPosition(pos.x, pos.z, pos.y);
                mesh.setMatrixAt(count, matrix);
                count++;
            }
        }
        mesh.count = count;
        mesh.instanceMatrix.needsUpdate = true;
    }
}

function clearMeshes() {
    for (const mesh of meshes) {
        scene.remove(mesh);
        mesh.geometry.dispose();
        mesh.material.dispose();
    }
    meshes = [];
}

export function dispose3D() {
    if (animationId) {
        cancelAnimationFrame(animationId);
        animationId = null;
    }

    clearMeshes();

    document.removeEventListener('keydown', onKeyDown);
    document.removeEventListener('keyup', onKeyUp);
    window.removeEventListener('resize', onResize);

    if (renderer && renderer.domElement) {
        renderer.domElement.removeEventListener('touchstart', onTouchStart);
        renderer.domElement.removeEventListener('touchmove', onTouchMove);
        renderer.domElement.removeEventListener('touchend', onTouchEnd);
    }

    if (controls) {
        if (controls.isLocked) controls.unlock();
        controls.dispose();
        controls = null;
    }

    if (renderer) {
        renderer.dispose();
        if (renderer.domElement && renderer.domElement.parentNode) {
            renderer.domElement.parentNode.removeChild(renderer.domElement);
        }
        renderer = null;
    }

    if (overlayEl && overlayEl.parentNode) {
        overlayEl.parentNode.removeChild(overlayEl);
        overlayEl = null;
    }
    if (hintsEl && hintsEl.parentNode) {
        hintsEl.parentNode.removeChild(hintsEl);
        hintsEl = null;
    }
    clearTimeout(hintsTimeout);

    // Reset movement
    moveState.forward = moveState.backward = moveState.left = moveState.right = moveState.up = moveState.down = false;
    touchLook = null;
    touchMove = null;

    scene = null;
    camera = null;
    container = null;
}

export function isInitialized() {
    return scene !== null && renderer !== null;
}
