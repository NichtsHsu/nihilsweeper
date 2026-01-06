use core::f32;

use crate::base::{Vec2D, board};
use itertools::iproduct;

pub mod brute_force;
pub mod error;
pub mod guessing;
pub mod half_chance;
pub mod probability;
pub mod trivial;

pub fn default_engine() -> impl AnalysisEngine {
    trivial::TrivialAnalysis::new(false)
        .then(probability::ProbabilityCalculator::new(false))
        .then(half_chance::HalfChanceCheck)
        .or(select(
            |board| board.conditions_more_than(1000.0),
            guessing::GuessingLogic,
            brute_force::BruteForceAnalysis,
        ))
}

#[derive(Debug, Clone, Copy)]
pub struct CellProbability {
    pub mine_probability: f32,
    pub second_safety: f32,
    pub progress_rate: f32,
    pub solve_rate: f32,
}

impl Default for CellProbability {
    fn default() -> Self {
        CellProbability {
            mine_probability: f32::NAN,
            second_safety: f32::NAN,
            progress_rate: f32::NAN,
            solve_rate: f32::NAN,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum CellSafety {
    #[default]
    // An unresolved closed cell that is not adjacent to any revealed numbers.
    Wilderness,
    // An unresolved closed cell that is adjacent to revealed number(s).
    Frontier,
    // A number cell that has adjacent unresolved cells.
    Unresolved(u8),
    // A number cell that has had all its adjacent mines or safe cells identified.
    // An empty cell always be `Resolved(0)`.
    Resolved(u8),
    // A closed cell that has been determined to be safe.
    Safe,
    // A closed cell that has been determined to be a mine.
    Mine,
    // A closed cell with an associated probability of being a mine.
    Probability(CellProbability),
}

#[derive(Debug, Clone)]
pub struct BoardSafety {
    cells: Vec<CellSafety>,
    width: usize,
    height: usize,
    mines: usize,
    suggestion: Option<(usize, usize)>,
}

impl BoardSafety {
    pub fn new(cell_states: &Vec2D<board::CellState>, mines: usize, admit_flags: bool) -> Self {
        let check_frontier = |x: usize, y: usize| {
            for nx in x.saturating_sub(1)..=(x + 1).min(cell_states.dims().0 - 1) {
                for ny in y.saturating_sub(1)..=(y + 1).min(cell_states.dims().1 - 1) {
                    if nx == x && ny == y {
                        continue;
                    }
                    if let board::CellState::Opening(_) = cell_states[(nx, ny)] {
                        return CellSafety::Frontier;
                    }
                }
            }
            CellSafety::Wilderness
        };

        let cells = iproduct![0..cell_states.dims().1, 0..cell_states.dims().0]
            .map(|(y, x)| match cell_states[(x, y)] {
                board::CellState::Opening(0) => CellSafety::Resolved(0),
                board::CellState::Opening(number) => CellSafety::Unresolved(number),
                board::CellState::Flagged if admit_flags => CellSafety::Mine,
                _ => check_frontier(x, y),
            })
            .collect();

        BoardSafety {
            cells,
            width: cell_states.dims().0,
            height: cell_states.dims().1,
            mines,
            suggestion: None,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn mines(&self) -> usize {
        self.mines
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&CellSafety> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.cells.get(y * self.width + x)
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut CellSafety> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.cells.get_mut(y * self.width + x)
    }

    pub fn set(&mut self, x: usize, y: usize, value: CellSafety) {
        if x >= self.width || y >= self.height {
            return;
        }
        if let Some(cell) = self.cells.get_mut(y * self.width + x) {
            *cell = value;
        }
    }

    pub fn suggestion(&self) -> Option<(usize, usize)> {
        self.suggestion
    }

    pub fn suggest(&mut self, x: usize, y: usize) {
        self.suggestion = Some((x, y));
    }

    pub fn conditions_more_than(&self, count: f64) -> bool {
        let mut unconfirmed: usize = 0;
        let mut remaining_mines = self.mines;
        for cell in &self.cells {
            match cell {
                CellSafety::Wilderness | CellSafety::Frontier | CellSafety::Probability(..) => unconfirmed += 1,
                CellSafety::Mine => remaining_mines -= 1,
                _ => {},
            }
        }
        let Some(remaining_safe) = unconfirmed.checked_sub(remaining_mines) else {
            return false;
        };
        let n = remaining_mines.min(remaining_safe);
        let mut conditions = 1.0f64;
        for i in 0..n {
            conditions /= i as f64;
            conditions *= (unconfirmed - n + i) as f64;
            if conditions > count {
                return true;
            }
        }
        false
    }
}

pub trait AnalysisEngine: Send + Sync {
    fn calculate(&self, board: BoardSafety) -> error::Result<BoardSafety>;
}

#[derive(Debug, Clone)]
struct AnalysisEngineCombinerAnd<T: AnalysisEngine, U: AnalysisEngine>(T, U);

impl<T: AnalysisEngine, U: AnalysisEngine> AnalysisEngine for AnalysisEngineCombinerAnd<T, U> {
    fn calculate(&self, board: BoardSafety) -> error::Result<BoardSafety> {
        self.1.calculate(self.0.calculate(board)?)
    }
}

#[derive(Debug, Clone)]
struct AnalysisEngineCombinerOr<T: AnalysisEngine, U: AnalysisEngine>(T, U);

impl<T: AnalysisEngine, U: AnalysisEngine> AnalysisEngine for AnalysisEngineCombinerOr<T, U> {
    fn calculate(&self, board: BoardSafety) -> error::Result<BoardSafety> {
        let board = self.0.calculate(board)?;
        if let Some((..)) = board.suggestion() {
            Ok(board)
        } else {
            self.1.calculate(board)
        }
    }
}

#[derive(Debug, Clone)]
struct AnalysisEngineCombinerSelect<F, T, U>
where
    F: Fn(&BoardSafety) -> bool + Send + Sync + 'static,
    T: AnalysisEngine + 'static,
    U: AnalysisEngine + 'static,
{
    condition: F,
    yes: T,
    no: U,
}

impl<F, T, U> AnalysisEngine for AnalysisEngineCombinerSelect<F, T, U>
where
    F: Fn(&BoardSafety) -> bool + Send + Sync + 'static,
    T: AnalysisEngine + 'static,
    U: AnalysisEngine + 'static,
{
    fn calculate(&self, board: BoardSafety) -> error::Result<BoardSafety> {
        if (self.condition)(&board) {
            self.yes.calculate(board)
        } else {
            self.no.calculate(board)
        }
    }
}

pub trait AnalysisEngineExt {
    /// Combines two analysis engines in sequence, passing the output of the first as input to the
    /// second. Suggestions made by the first engine may be either ignored or accepted by the
    /// second.
    fn then<T: AnalysisEngine>(self, next: T) -> impl AnalysisEngine
    where
        Self: Sized + AnalysisEngine,
    {
        AnalysisEngineCombinerAnd(self, next)
    }

    /// Combines two analysis engines such that the second is only invoked if the first does not
    /// produce a suggestion.
    fn or<T: AnalysisEngine>(self, alternative: T) -> impl AnalysisEngine
    where
        Self: Sized + AnalysisEngine,
    {
        AnalysisEngineCombinerOr(self, alternative)
    }
}

/// Selects between two analysis engines based on a condition evaluated on the input board.
pub fn select<F, T, U>(condition: F, yes: T, no: U) -> impl AnalysisEngine
where
    F: Fn(&BoardSafety) -> bool + Send + Sync + 'static,
    T: AnalysisEngine + 'static,
    U: AnalysisEngine + 'static,
{
    AnalysisEngineCombinerSelect { condition, yes, no }
}

impl<T: AnalysisEngine> AnalysisEngineExt for T {}
