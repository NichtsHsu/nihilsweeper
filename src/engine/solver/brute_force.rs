use super::{BoardSafety, CellSafety, Solver};

#[derive(Debug, Clone, Default)]
pub struct BruteForceSolver;

impl Solver for BruteForceSolver {
    fn calculate(&self, board: BoardSafety) -> super::error::Result<BoardSafety> {
        Ok(board)
    }
}
