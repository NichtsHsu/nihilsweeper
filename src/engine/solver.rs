use core::f32;
use std::ops::{Deref, DerefMut};

use crate::base::{Vec2D, board};

pub mod brute_force;
pub mod error;
pub mod guessing;
pub mod half_chance;
pub mod probability;
pub mod trivial;

pub fn default_engine() -> impl Solver {
    trivial::TrivialSolver::new(false)
        .then(probability::ProbabilityCalculator::new(false))
        .or(half_chance::HalfChanceCheck)
        .or(select(
            |board| board.conditions_more_than(1000.0),
            guessing::GuessingLogic,
            brute_force::BruteForceSolver,
        ))
}

#[derive(Debug, Clone, Copy)]
pub struct CellProbability {
    pub frontier: bool,
    pub mine_probability: f32,
    pub second_safety: f32,
    pub progress_rate: f32,
    pub solve_rate: f32,
}

impl Default for CellProbability {
    fn default() -> Self {
        CellProbability {
            frontier: false,
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
    /// An unsolved closed cell that is not adjacent to any revealed numbers.
    Wilderness,
    /// An unsolved closed cell that is adjacent to revealed number(s).
    Frontier,
    /// A number cell that has adjacent unsolved cells.
    Unsolved(u8),
    /// A number cell that has had all its adjacent mines or safe cells identified.
    /// An empty cell always be `Solved(0)`.
    Solved(u8),
    /// A closed cell that has been determined to be safe.
    Safe,
    /// A closed cell that has been determined to be a mine.
    Mine,
    /// A closed cell with an associated probability of being a mine.
    Probability(CellProbability),
}

#[derive(Debug, Clone)]
pub struct BoardSafety {
    cells: Vec2D<CellSafety>,
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

        let mut cells = Vec2D::new(cell_states.dims().0, cell_states.dims().1);
        for y in 0..cell_states.dims().1 {
            for x in 0..cell_states.dims().0 {
                cells[(x, y)] = match cell_states[(x, y)] {
                    board::CellState::Opening(0) => CellSafety::Solved(0),
                    board::CellState::Opening(number) => CellSafety::Unsolved(number),
                    board::CellState::Flagged if admit_flags => CellSafety::Mine,
                    _ => check_frontier(x, y),
                };
            }
        }

        BoardSafety {
            cells,
            mines,
            suggestion: None,
        }
    }

    pub fn width(&self) -> usize {
        self.cells.dims().0
    }

    pub fn height(&self) -> usize {
        self.cells.dims().1
    }

    pub fn mines(&self) -> usize {
        self.mines
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
        for cell in self.cells.data() {
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

impl Deref for BoardSafety {
    type Target = Vec2D<CellSafety>;

    fn deref(&self) -> &Self::Target {
        &self.cells
    }
}

impl DerefMut for BoardSafety {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cells
    }
}

pub trait Solver: Send + Sync {
    fn calculate(&self, board: BoardSafety) -> error::Result<BoardSafety>;
}

#[derive(Debug, Clone)]
struct SolverCombinerAnd<T: Solver, U: Solver>(T, U);

impl<T: Solver, U: Solver> Solver for SolverCombinerAnd<T, U> {
    fn calculate(&self, board: BoardSafety) -> error::Result<BoardSafety> {
        self.1.calculate(self.0.calculate(board)?)
    }
}

#[derive(Debug, Clone)]
struct SolverCombinerOr<T: Solver, U: Solver>(T, U);

impl<T: Solver, U: Solver> Solver for SolverCombinerOr<T, U> {
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
struct SolverCombinerSelect<F, T, U>
where
    F: Fn(&BoardSafety) -> bool + Send + Sync + 'static,
    T: Solver + 'static,
    U: Solver + 'static,
{
    condition: F,
    yes: T,
    no: U,
}

impl<F, T, U> Solver for SolverCombinerSelect<F, T, U>
where
    F: Fn(&BoardSafety) -> bool + Send + Sync + 'static,
    T: Solver + 'static,
    U: Solver + 'static,
{
    fn calculate(&self, board: BoardSafety) -> error::Result<BoardSafety> {
        if (self.condition)(&board) {
            self.yes.calculate(board)
        } else {
            self.no.calculate(board)
        }
    }
}

pub trait SolverExt {
    /// Combines two solver engines in sequence, passing the output of the first as input to the
    /// second. Suggestions made by the first engine may be either ignored or accepted by the
    /// second.
    fn then<T: Solver>(self, next: T) -> impl Solver
    where
        Self: Sized + Solver,
    {
        SolverCombinerAnd(self, next)
    }

    /// Combines two solver engines such that the second is only invoked if the first does not
    /// produce a suggestion.
    fn or<T: Solver>(self, alternative: T) -> impl Solver
    where
        Self: Sized + Solver,
    {
        SolverCombinerOr(self, alternative)
    }
}

/// Selects between two solver engines based on a condition evaluated on the input board.
pub fn select<F, T, U>(condition: F, yes: T, no: U) -> impl Solver
where
    F: Fn(&BoardSafety) -> bool + Send + Sync + 'static,
    T: Solver + 'static,
    U: Solver + 'static,
{
    SolverCombinerSelect { condition, yes, no }
}

impl<T: Solver> SolverExt for T {}
