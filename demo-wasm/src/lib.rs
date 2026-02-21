use wasm_bindgen::prelude::*;
use wavfc::*;
use wavfc::constraint::GlobalConstraint;

use rand::rngs::SmallRng;
use rand::SeedableRng;

mod tilesets;
mod topology_2d;
mod topology_3d;

use tilesets::{DungeonRules, ForestVoxelRules, TerrainRules, TerrainVoxelRules, TimeEvolutionRules, VillageVoxelRules};
use topology_2d::Grid2D;
use topology_3d::Grid3D;

// ---------------------------------------------------------------------------
// Full solve (2D)
// ---------------------------------------------------------------------------

/// Generates a fully solved WFC grid and returns a flat array of tile states.
///
/// Each element is a `u8` representing the tile state at that cell index
/// (row-major order).
#[wasm_bindgen]
pub fn generate(
    width: usize,
    height: usize,
    seed: u64,
    tileset: &str,
    wrapping: bool,
    propagator: &str,
    heuristic: &str,
    backtrack_mode: &str,
    backtrack_param: usize,
) -> Result<Vec<u8>, JsValue> {
    let grid = Grid2D {
        width,
        height,
        wrapping,
    };

    let prop = match propagator {
        "ac4" => PropagatorKind::Ac4,
        _ => PropagatorKind::Ac3,
    };
    let heur = match heuristic {
        "shannon" => Heuristic::ShannonEntropy,
        _ => Heuristic::MinCount,
    };
    let bt = match backtrack_mode {
        "restart" => BacktrackStrategy::Restart {
            max_restarts: backtrack_param,
        },
        "chronological" => BacktrackStrategy::Chronological {
            max_depth: backtrack_param,
        },
        _ => BacktrackStrategy::None,
    };

    let config = SolverConfig::new()
        .propagator(prop)
        .heuristic(heur)
        .backtrack(bt);

    let mut rng = SmallRng::seed_from_u64(seed);

    match tileset {
        "dungeon" => {
            let rules = DungeonRules;
            let mut solver = WfcSolver::new(grid, &rules, config)
                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
            let result = solver
                .solve(&mut rng)
                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
            Ok(result.states().iter().map(|&s| s as u8).collect())
        }
        _ => {
            let rules = TerrainRules;
            let mut solver = WfcSolver::new(grid, &rules, config)
                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
            let result = solver
                .solve(&mut rng)
                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
            Ok(result.states().iter().map(|&s| s as u8).collect())
        }
    }
}

// ---------------------------------------------------------------------------
// Timed generation (2D)
// ---------------------------------------------------------------------------

/// Like `generate`, but also returns timing information as a JSON string.
///
/// Returns a JSON string: `{"states":[...],"elapsed_ms":...,"width":...,"height":...}`
#[wasm_bindgen]
pub fn generate_timed(
    width: usize,
    height: usize,
    seed: u64,
    tileset: &str,
    wrapping: bool,
    propagator: &str,
    heuristic: &str,
    backtrack_mode: &str,
    backtrack_param: usize,
) -> Result<JsValue, JsValue> {
    let window = web_sys::window().unwrap();
    let perf = window.performance().unwrap();

    let start = perf.now();
    let states = generate(
        width,
        height,
        seed,
        tileset,
        wrapping,
        propagator,
        heuristic,
        backtrack_mode,
        backtrack_param,
    )?;
    let elapsed = perf.now() - start;

    let states_str: String = states
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let json = format!(
        r#"{{"states":[{states_str}],"elapsed_ms":{elapsed:.2},"width":{width},"height":{height}}}"#,
    );

    Ok(JsValue::from_str(&json))
}

// ---------------------------------------------------------------------------
// Step-through solver (2D)
// ---------------------------------------------------------------------------

enum SolverVariant {
    Terrain(WfcSolver<Grid2D>),
    Dungeon(WfcSolver<Grid2D>),
}

/// A step-through WFC solver for animated visualization.
///
/// Call `step()` repeatedly to advance the solve one cell at a time.
#[wasm_bindgen]
pub struct StepSolver {
    inner: SolverVariant,
    width: usize,
    height: usize,
    rng: SmallRng,
    tileset: String,
    states: Vec<u8>,
}

#[wasm_bindgen]
impl StepSolver {
    /// Creates a new step-through solver.
    #[wasm_bindgen(constructor)]
    pub fn new(
        width: usize,
        height: usize,
        seed: u64,
        tileset: &str,
        wrapping: bool,
        propagator: &str,
        heuristic: &str,
    ) -> Result<StepSolver, JsValue> {
        let grid = Grid2D {
            width,
            height,
            wrapping,
        };
        let prop = match propagator {
            "ac4" => PropagatorKind::Ac4,
            _ => PropagatorKind::Ac3,
        };
        let heur = match heuristic {
            "shannon" => Heuristic::ShannonEntropy,
            _ => Heuristic::MinCount,
        };
        let config = SolverConfig::new().propagator(prop).heuristic(heur);

        let rng = SmallRng::seed_from_u64(seed);

        let states = vec![255u8; width * height];

        let inner = match tileset {
            "dungeon" => {
                let rules = DungeonRules;
                let solver = WfcSolver::new(grid, &rules, config)
                    .map_err(|e| JsValue::from_str(&format!("{e}")))?;
                SolverVariant::Dungeon(solver)
            }
            _ => {
                let rules = TerrainRules;
                let solver = WfcSolver::new(grid, &rules, config)
                    .map_err(|e| JsValue::from_str(&format!("{e}")))?;
                SolverVariant::Terrain(solver)
            }
        };

        Ok(StepSolver {
            inner,
            width,
            height,
            rng,
            tileset: tileset.to_string(),
            states,
        })
    }

    /// Advances the solver by one step.
    ///
    /// Returns a JSON string describing what happened:
    /// - `{"type":"collapsed","cell":N,"state":S}` -- a cell was collapsed
    /// - `{"type":"contradiction","cell":N}` -- a contradiction was reached
    /// - `{"type":"complete"}` -- the solve is finished
    pub fn step(&mut self) -> String {
        let result = match &mut self.inner {
            SolverVariant::Terrain(s) => s.step(&mut self.rng),
            SolverVariant::Dungeon(s) => s.step(&mut self.rng),
        };
        match result {
            StepResult::Collapsed { cell, state } => {
                self.states[cell] = state as u8;
                format!(r#"{{"type":"collapsed","cell":{cell},"state":{state}}}"#)
            }
            StepResult::Contradiction { cell } => {
                format!(r#"{{"type":"contradiction","cell":{cell}}}"#)
            }
            StepResult::Complete => r#"{"type":"complete"}"#.to_string(),
        }
    }

    /// Returns the current grid state as a flat `Vec<u8>`.
    ///
    /// Uncollapsed cells have value 255.
    pub fn get_current_state(&self) -> Vec<u8> {
        self.states.clone()
    }

    /// Returns the hex color string for a tile state in the current tileset.
    pub fn get_color(&self, state: u8) -> String {
        match self.tileset.as_str() {
            "dungeon" => tilesets::dungeon_color(state as usize).to_string(),
            _ => tilesets::terrain_color(state as usize).to_string(),
        }
    }

    /// Returns the human-readable name for a tile state in the current tileset.
    pub fn get_state_name(&self, state: u8) -> String {
        match self.tileset.as_str() {
            "dungeon" => tilesets::dungeon_name(state as usize).to_string(),
            _ => tilesets::terrain_name(state as usize).to_string(),
        }
    }

    /// Grid width.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Grid height.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Number of tile states in the current tileset.
    pub fn num_states(&self) -> usize {
        match self.tileset.as_str() {
            "dungeon" => 5,
            _ => 7,
        }
    }
}

// ---------------------------------------------------------------------------
// Full solve (3D)
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn generate_3d(
    width: usize,
    height: usize,
    depth: usize,
    seed: u64,
    tileset_3d: &str,
    wrapping: bool,
    propagator: &str,
    heuristic: &str,
    backtrack_mode: &str,
    backtrack_param: usize,
) -> Result<Vec<u8>, JsValue> {
    let grid = Grid3D {
        width,
        height,
        depth,
        wrapping,
    };

    let prop = match propagator {
        "ac4" => PropagatorKind::Ac4,
        _ => PropagatorKind::Ac3,
    };
    let heur = match heuristic {
        "shannon" => Heuristic::ShannonEntropy,
        _ => Heuristic::MinCount,
    };
    let bt = match backtrack_mode {
        "restart" => BacktrackStrategy::Restart {
            max_restarts: backtrack_param,
        },
        "chronological" => BacktrackStrategy::Chronological {
            max_depth: backtrack_param,
        },
        _ => BacktrackStrategy::None,
    };

    // For constrained tilesets, default to restart backtracking if user selected "none"
    let constrained_bt = match bt {
        BacktrackStrategy::None => BacktrackStrategy::Restart { max_restarts: 20 },
        other => other,
    };

    let config = SolverConfig::new()
        .propagator(prop)
        .heuristic(heur)
        .backtrack(bt);

    let constrained_config = SolverConfig::new()
        .propagator(prop)
        .heuristic(heur)
        .backtrack(constrained_bt);

    let mut rng = SmallRng::seed_from_u64(seed);

    let total = width * height * depth;

    match tileset_3d {
        "forest" => {
            let freq = FrequencyConstraint::builder()
                .max_fraction(6, 0.03, total)   // Trunk: max 3%
                .max_fraction(7, 0.08, total)   // Leaves: max 8%
                .build();
            let run = MaxRunConstraint::builder()
                .max_run(6, 3)   // Trunk: max 3 consecutive
                .max_run(7, 4)   // Leaves: max 4 consecutive
                .build(8);
            let constraints: &[&dyn GlobalConstraint<Grid3D>] = &[&freq, &run];
            let rules = ForestVoxelRules;
            let mut solver = WfcSolver::new(grid, &rules, constrained_config)
                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
            let result = solver
                .solve_with(&mut rng, &mut NoOpObserver, constraints)
                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
            Ok(result.states().iter().map(|&s| s as u8).collect())
        }
        "village" => {
            let freq = FrequencyConstraint::builder()
                .max_fraction(6, 0.03, total)   // Trunk
                .max_fraction(7, 0.06, total)   // Leaves
                .max_fraction(8, 0.06, total)   // Wall
                .max_fraction(9, 0.04, total)   // Floor
                .max_fraction(10, 0.03, total)  // Roof
                .build();
            let run = MaxRunConstraint::builder()
                .max_run(6, 3)    // Trunk
                .max_run(7, 4)    // Leaves
                .max_run(8, 4)    // Wall
                .max_run(9, 3)    // Floor
                .build(11);
            let constraints: &[&dyn GlobalConstraint<Grid3D>] = &[&freq, &run];
            let rules = VillageVoxelRules;
            let mut solver = WfcSolver::new(grid, &rules, constrained_config)
                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
            let result = solver
                .solve_with(&mut rng, &mut NoOpObserver, constraints)
                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
            Ok(result.states().iter().map(|&s| s as u8).collect())
        }
        "time_evolution" => {
            let rules = TimeEvolutionRules;
            let mut solver = WfcSolver::new(grid, &rules, config)
                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
            let result = solver
                .solve(&mut rng)
                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
            Ok(result.states().iter().map(|&s| s as u8).collect())
        }
        _ => {
            // terrain_3d
            let rules = TerrainVoxelRules;
            let mut solver = WfcSolver::new(grid, &rules, config)
                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
            let result = solver
                .solve(&mut rng)
                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
            Ok(result.states().iter().map(|&s| s as u8).collect())
        }
    }
}

// ---------------------------------------------------------------------------
// Timed generation (3D)
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn generate_3d_timed(
    width: usize,
    height: usize,
    depth: usize,
    seed: u64,
    tileset_3d: &str,
    wrapping: bool,
    propagator: &str,
    heuristic: &str,
    backtrack_mode: &str,
    backtrack_param: usize,
) -> Result<JsValue, JsValue> {
    let window = web_sys::window().unwrap();
    let perf = window.performance().unwrap();

    let start = perf.now();
    let states = generate_3d(
        width,
        height,
        depth,
        seed,
        tileset_3d,
        wrapping,
        propagator,
        heuristic,
        backtrack_mode,
        backtrack_param,
    )?;
    let elapsed = perf.now() - start;

    let states_str: String = states
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let json = format!(
        r#"{{"states":[{states_str}],"elapsed_ms":{elapsed:.2},"width":{width},"height":{height},"depth":{depth}}}"#,
    );

    Ok(JsValue::from_str(&json))
}

// ---------------------------------------------------------------------------
// Step-through solver (3D)
// ---------------------------------------------------------------------------

enum SolverVariant3D {
    Terrain(WfcSolver<Grid3D>),
    Forest(WfcSolver<Grid3D>),
    Village(WfcSolver<Grid3D>),
    TimeEvolution(WfcSolver<Grid3D>),
}

#[wasm_bindgen]
pub struct StepSolver3D {
    inner: SolverVariant3D,
    width: usize,
    height: usize,
    depth: usize,
    rng: SmallRng,
    tileset: String,
    states: Vec<u8>,
}

#[wasm_bindgen]
impl StepSolver3D {
    #[wasm_bindgen(constructor)]
    pub fn new(
        width: usize,
        height: usize,
        depth: usize,
        seed: u64,
        tileset_3d: &str,
        wrapping: bool,
        propagator: &str,
        heuristic: &str,
    ) -> Result<StepSolver3D, JsValue> {
        let grid = Grid3D {
            width,
            height,
            depth,
            wrapping,
        };
        let prop = match propagator {
            "ac4" => PropagatorKind::Ac4,
            _ => PropagatorKind::Ac3,
        };
        let heur = match heuristic {
            "shannon" => Heuristic::ShannonEntropy,
            _ => Heuristic::MinCount,
        };
        let config = SolverConfig::new().propagator(prop).heuristic(heur);

        let rng = SmallRng::seed_from_u64(seed);

        let states = vec![255u8; width * height * depth];

        let inner = match tileset_3d {
            "forest" => {
                let rules = ForestVoxelRules;
                let solver = WfcSolver::new(grid, &rules, config)
                    .map_err(|e| JsValue::from_str(&format!("{e}")))?;
                SolverVariant3D::Forest(solver)
            }
            "village" => {
                let rules = VillageVoxelRules;
                let solver = WfcSolver::new(grid, &rules, config)
                    .map_err(|e| JsValue::from_str(&format!("{e}")))?;
                SolverVariant3D::Village(solver)
            }
            "time_evolution" => {
                let rules = TimeEvolutionRules;
                let solver = WfcSolver::new(grid, &rules, config)
                    .map_err(|e| JsValue::from_str(&format!("{e}")))?;
                SolverVariant3D::TimeEvolution(solver)
            }
            _ => {
                let rules = TerrainVoxelRules;
                let solver = WfcSolver::new(grid, &rules, config)
                    .map_err(|e| JsValue::from_str(&format!("{e}")))?;
                SolverVariant3D::Terrain(solver)
            }
        };

        Ok(StepSolver3D {
            inner,
            width,
            height,
            depth,
            rng,
            tileset: tileset_3d.to_string(),
            states,
        })
    }

    pub fn step(&mut self) -> String {
        let result = match &mut self.inner {
            SolverVariant3D::Terrain(s) => s.step(&mut self.rng),
            SolverVariant3D::Forest(s) => s.step(&mut self.rng),
            SolverVariant3D::Village(s) => s.step(&mut self.rng),
            SolverVariant3D::TimeEvolution(s) => s.step(&mut self.rng),
        };
        match result {
            StepResult::Collapsed { cell, state } => {
                self.states[cell] = state as u8;
                format!(r#"{{"type":"collapsed","cell":{cell},"state":{state}}}"#)
            }
            StepResult::Contradiction { cell } => {
                format!(r#"{{"type":"contradiction","cell":{cell}}}"#)
            }
            StepResult::Complete => r#"{"type":"complete"}"#.to_string(),
        }
    }

    pub fn get_current_state(&self) -> Vec<u8> {
        self.states.clone()
    }

    pub fn get_color(&self, state: u8) -> String {
        match self.tileset.as_str() {
            "forest" => tilesets::voxel_forest_color(state as usize).to_string(),
            "village" => tilesets::voxel_village_color(state as usize).to_string(),
            "time_evolution" => tilesets::time_evolution_color(state as usize).to_string(),
            _ => tilesets::voxel_terrain_color(state as usize).to_string(),
        }
    }

    pub fn get_state_name(&self, state: u8) -> String {
        match self.tileset.as_str() {
            "forest" => tilesets::voxel_forest_name(state as usize).to_string(),
            "village" => tilesets::voxel_village_name(state as usize).to_string(),
            "time_evolution" => tilesets::time_evolution_name(state as usize).to_string(),
            _ => tilesets::voxel_terrain_name(state as usize).to_string(),
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn num_states(&self) -> usize {
        match self.tileset.as_str() {
            "forest" => 8,
            "village" => 11,
            "time_evolution" => 6,
            _ => 6,
        }
    }
}

// ---------------------------------------------------------------------------
// Standalone color / name helpers
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn get_terrain_color(state: u8) -> String {
    tilesets::terrain_color(state as usize).to_string()
}

#[wasm_bindgen]
pub fn get_dungeon_color(state: u8) -> String {
    tilesets::dungeon_color(state as usize).to_string()
}

#[wasm_bindgen]
pub fn get_terrain_name(state: u8) -> String {
    tilesets::terrain_name(state as usize).to_string()
}

#[wasm_bindgen]
pub fn get_dungeon_name(state: u8) -> String {
    tilesets::dungeon_name(state as usize).to_string()
}

#[wasm_bindgen]
pub fn get_voxel_color(tileset: &str, state: u8) -> String {
    match tileset {
        "forest" => tilesets::voxel_forest_color(state as usize).to_string(),
        "village" => tilesets::voxel_village_color(state as usize).to_string(),
        "time_evolution" => tilesets::time_evolution_color(state as usize).to_string(),
        _ => tilesets::voxel_terrain_color(state as usize).to_string(),
    }
}

#[wasm_bindgen]
pub fn get_voxel_name(tileset: &str, state: u8) -> String {
    match tileset {
        "forest" => tilesets::voxel_forest_name(state as usize).to_string(),
        "village" => tilesets::voxel_village_name(state as usize).to_string(),
        "time_evolution" => tilesets::time_evolution_name(state as usize).to_string(),
        _ => tilesets::voxel_terrain_name(state as usize).to_string(),
    }
}

#[wasm_bindgen]
pub fn get_voxel_num_states(tileset: &str) -> usize {
    match tileset {
        "forest" => 8,
        "village" => 11,
        "time_evolution" => 6,
        _ => 6,
    }
}

#[wasm_bindgen]
pub fn get_num_states(tileset: &str) -> usize {
    match tileset {
        "dungeon" => 5,
        "terrain_3d" => 6,
        "forest" => 8,
        "village" => 11,
        "time_evolution" => 6,
        _ => 7,
    }
}
