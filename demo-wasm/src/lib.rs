use wasm_bindgen::prelude::*;
use wavfc::*;

use rand::rngs::SmallRng;
use rand::SeedableRng;

mod tilesets;
mod topology_2d;

use tilesets::{DungeonRules, TerrainRules};
use topology_2d::Grid2D;

// ---------------------------------------------------------------------------
// Full solve
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
// Timed generation
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
// Step-through solver
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
pub fn get_num_states(tileset: &str) -> usize {
    match tileset {
        "dungeon" => 5,
        _ => 7,
    }
}
