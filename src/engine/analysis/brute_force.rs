use super::{AnalysisEngine, BoardSafety, CellSafety};

#[derive(Debug, Clone, Default)]
pub struct BruteForceAnalysis;

impl AnalysisEngine for BruteForceAnalysis {
    fn calculate(&self, board: BoardSafety) -> super::error::Result<BoardSafety> {
        Ok(board)
    }
}
