use super::{AnalysisEngine, BoardSafety, CellSafety};

#[derive(Debug, Clone, Default)]
pub struct GuessingLogic;

impl AnalysisEngine for GuessingLogic {
    fn calculate(&self, board: BoardSafety) -> super::error::Result<BoardSafety> {
        Ok(board)
    }
}
