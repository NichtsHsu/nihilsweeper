use log::trace;

use super::{AnalysisEngine, BoardSafety, CellSafety};

#[derive(Debug, Clone, Default)]
pub struct TrivialAnalysis {
    stop_on_first_safe: bool,
}

impl TrivialAnalysis {
    pub fn new(stop_on_first_safe: bool) -> Self {
        Self { stop_on_first_safe }
    }

    /// Analyze a single position on the board.
    /// Returns `Ok(true)` if `stop_on_first_safe` is set and a safe cell is found.
    fn calculate_position(&self, board: &mut BoardSafety, x: usize, y: usize) -> super::error::Result<bool> {
        if let Some(&CellSafety::Unresolved(n)) = board.get(x, y) {
            let mut flagged_neighbors = 0;
            let mut unopened_neighbors = 0;
            for nx in x.saturating_sub(1)..=(x + 1).min(board.width() - 1) {
                for ny in y.saturating_sub(1)..=(y + 1).min(board.height() - 1) {
                    if nx == x && ny == y {
                        continue;
                    }
                    match board.get(nx, ny) {
                        Some(CellSafety::Mine) => flagged_neighbors += 1,
                        Some(CellSafety::Wilderness | CellSafety::Frontier | CellSafety::Probability { .. }) => {
                            unopened_neighbors += 1
                        },
                        _ => {},
                    }
                }
            }
            if flagged_neighbors == n && unopened_neighbors > 0 {
                // All adjacent unresolved cells are safe
                board.set(x, y, CellSafety::Resolved(n));
                for nx in x.saturating_sub(1)..=(x + 1).min(board.width() - 1) {
                    for ny in y.saturating_sub(1)..=(y + 1).min(board.height() - 1) {
                        if nx == x && ny == y {
                            continue;
                        }
                        if matches!(
                            board.get(nx, ny),
                            Some(CellSafety::Wilderness | CellSafety::Frontier | CellSafety::Probability { .. })
                        ) {
                            board.set(nx, ny, CellSafety::Safe);
                            if board.suggestion().is_none() {
                                board.suggest(nx, ny);
                                if self.stop_on_first_safe {
                                    return Ok(true);
                                }
                            }
                            // No need to check the return value here.
                            // If `stop_on_first_safe` is set, we will have already returned.
                            self.spread(board, nx, ny)?;
                        }
                    }
                }
            } else if flagged_neighbors + unopened_neighbors == n && unopened_neighbors > 0 {
                // All adjacent unresolved cells are mines
                board.set(x, y, CellSafety::Resolved(n));
                for nx in x.saturating_sub(1)..=(x + 1).min(board.width() - 1) {
                    for ny in y.saturating_sub(1)..=(y + 1).min(board.height() - 1) {
                        if nx == x && ny == y {
                            continue;
                        }
                        if matches!(
                            board.get(nx, ny),
                            Some(CellSafety::Wilderness | CellSafety::Frontier | CellSafety::Probability { .. })
                        ) {
                            board.set(nx, ny, CellSafety::Mine);

                            if self.spread(board, nx, ny)? {
                                return Ok(true);
                            }
                        }
                    }
                }
            } else if flagged_neighbors > n {
                return Err(super::error::Error::MinesNotSatisfied {
                    x,
                    y,
                    expected: n,
                    actual: flagged_neighbors,
                });
            } else if flagged_neighbors + unopened_neighbors < n {
                return Err(super::error::Error::MinesNotSatisfied {
                    x,
                    y,
                    expected: n,
                    actual: flagged_neighbors + unopened_neighbors,
                });
            };
        }
        Ok(false)
    }

    /// When a cell is marked as safe or a mine, spread the analysis to its neighbors.
    /// Returns `Ok(true)` if `stop_on_first_safe` is set and a safe cell is found.
    fn spread(&self, board: &mut BoardSafety, x: usize, y: usize) -> super::error::Result<bool> {
        for nx in x.saturating_sub(1)..=(x + 1).min(board.width() - 1) {
            for ny in y.saturating_sub(1)..=(y + 1).min(board.height() - 1) {
                if nx == x && ny == y {
                    continue;
                }
                if self.calculate_position(board, nx, ny)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

impl AnalysisEngine for TrivialAnalysis {
    fn calculate(&self, mut board: BoardSafety) -> super::error::Result<BoardSafety> {
        'stop: for x in 0..board.width() {
            for y in 0..board.height() {
                if self.calculate_position(&mut board, x, y)? {
                    trace!(
                        "TrivialAnalysis: stopping on first safe cell at {:?}",
                        board.suggestion().unwrap()
                    );
                    break 'stop;
                }
            }
        }
        Ok(board)
    }
}
