use crate::{
    base::board,
    engine::analysis::{self, AnalysisEngine, BoardSafety},
};
use iced::widget::canvas;
use log::{error, trace};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum AnalysisOverlayMessage {
    AnalysisCompleted(analysis::error::Result<BoardSafety>),
    Resize {
        cell_size: u32,
        game_area: iced::Rectangle,
        board_area: iced::Rectangle,
    },
    SetEnabled(bool),
    SetAdmitFlags(bool),
}

pub struct AnalysisOverlay {
    enabled: bool,
    analysis_engine: Arc<dyn AnalysisEngine>,
    analysis_result: Option<BoardSafety>,
    analysis_admit_flags: bool,
    game_area: iced::Rectangle,
    board_area: iced::Rectangle,
    viewport: iced::Rectangle,
    cell_size: u32,
    cache: canvas::Cache,
}

impl AnalysisOverlay {
    pub fn new<T: AnalysisEngine + 'static>(
        analysis_engine: T,
        analysis_admit_flags: bool,
        game_area: iced::Rectangle,
        board_area: iced::Rectangle,
        viewport: iced::Rectangle,
        cell_size: u32,
        enabled: bool,
    ) -> Self {
        AnalysisOverlay {
            enabled,
            analysis_engine: Arc::new(analysis_engine),
            analysis_result: None,
            analysis_admit_flags,
            game_area,
            board_area,
            viewport,
            cell_size,
            cache: canvas::Cache::new(),
        }
    }

    fn cell_position(&self, x: usize, y: usize) -> iced::Point {
        iced::Point::new(
            x as f32 * self.cell_size as f32 + self.board_area.x,
            y as f32 * self.cell_size as f32 + self.board_area.y,
        )
    }

    pub fn set_viewport(&mut self, viewport: iced::Rectangle) {
        self.viewport = viewport;
        self.cache.clear();
    }

    pub fn update_analysis(&mut self, board: &dyn board::Board) -> iced::Task<AnalysisOverlayMessage> {
        if !self.enabled {
            return iced::Task::none();
        }
        let analysis_engine = Arc::clone(&self.analysis_engine);
        let board_safety = BoardSafety::new(board, self.analysis_admit_flags);

        iced::Task::perform(
            async move { analysis_engine.calculate(board_safety) },
            AnalysisOverlayMessage::AnalysisCompleted,
        )
    }

    pub fn clear_analysis(&mut self) {
        self.analysis_result = None;
        self.cache.clear();
    }

    pub fn update(&mut self, message: AnalysisOverlayMessage) {
        match message {
            AnalysisOverlayMessage::AnalysisCompleted(result) => {
                trace!("Analysis completed, updating overlay");
                self.analysis_result = result.inspect_err(|e| error!("Analysis error: {:?}", e)).ok();
                self.cache.clear();
            },
            AnalysisOverlayMessage::Resize {
                cell_size,
                game_area,
                board_area,
            } => {
                self.cell_size = cell_size;
                self.game_area = game_area;
                self.board_area = board_area;
                self.cache.clear();
            },
            AnalysisOverlayMessage::SetEnabled(enabled) => {
                self.enabled = enabled;
                if !enabled {
                    self.clear_analysis();
                }
            },
            AnalysisOverlayMessage::SetAdmitFlags(admit_flags) => {
                self.analysis_admit_flags = admit_flags;
                self.clear_analysis();
            },
        }
    }

    pub fn view(&self) -> iced::Element<'_, super::MainWindowMessage> {
        canvas::Canvas::new(self)
            .width(self.game_area.width)
            .height(self.game_area.height)
            .into()
    }
}

impl canvas::Program<super::MainWindowMessage> for AnalysisOverlay {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        if !self.enabled {
            return vec![];
        }

        let Some(board) = &self.analysis_result else {
            return vec![];
        };

        let geom = self.cache.draw(renderer, bounds.size(), |frame| {
            // Calculate visible cell range for viewport culling
            let cell_size_f32 = self.cell_size as f32;

            // The viewport from scrollable is in scrollable content coordinates
            // bounds.position() gives us the canvas position within the scrollable content
            // We need to calculate which part of the canvas (board_area) is visible
            let canvas_x = bounds.x;
            let canvas_y = bounds.y;

            // Calculate the intersection of viewport and board area
            // Adjust board_area coordinates to scrollable content space
            let board_x_in_content = canvas_x + self.board_area.x;
            let board_y_in_content = canvas_y + self.board_area.y;
            let board_x_end = board_x_in_content + self.board_area.width;
            let board_y_end = board_y_in_content + self.board_area.height;

            let visible_x_start = self.viewport.x.max(board_x_in_content);
            let visible_y_start = self.viewport.y.max(board_y_in_content);
            let visible_x_end = (self.viewport.x + self.viewport.width).min(board_x_end);
            let visible_y_end = (self.viewport.y + self.viewport.height).min(board_y_end);

            let visible_width = visible_x_end - visible_x_start;
            let visible_height = visible_y_end - visible_y_start;

            // Early return if viewport doesn't intersect with board area
            if visible_width <= 0.0 || visible_height <= 0.0 {
                trace!("Viewport doesn't intersect with board area, skipping cell rendering");
                return;
            }

            // Convert viewport bounds to cell coordinates (relative to board_area)
            let start_x = ((visible_x_start - board_x_in_content) / cell_size_f32).floor() as usize;
            let start_y = ((visible_y_start - board_y_in_content) / cell_size_f32).floor() as usize;

            let end_x_unclamped = ((visible_x_end - board_x_in_content) / cell_size_f32).ceil() as usize;
            let end_y_unclamped = ((visible_y_end - board_y_in_content) / cell_size_f32).ceil() as usize;
            let end_x = end_x_unclamped.min(board.width());
            let end_y = end_y_unclamped.min(board.height());

            trace!(
                "Viewport culling: canvas at ({}, {}), drawing cells from ({}, {}) to ({}, {}) out of board size {}x{}",
                canvas_x,
                canvas_y,
                start_x,
                start_y,
                end_x,
                end_y,
                board.width(),
                board.height()
            );

            for x in start_x..end_x {
                for y in start_y..end_y {
                    'no_overlay: {
                        if let Some(analysis) = &self.analysis_result
                            && let Some(cell_safety) = analysis.get(x, y)
                        {
                            let overlay_color = match cell_safety {
                                crate::engine::analysis::CellSafety::Safe => iced::Color::from_rgba(0.0, 1.0, 0.0, 0.5),
                                crate::engine::analysis::CellSafety::Mine => iced::Color::from_rgba(1.0, 0.0, 0.0, 0.5),
                                crate::engine::analysis::CellSafety::Probability(cell_probability) => {
                                    iced::Color::from_rgba(
                                        cell_probability.mine_probability / 100.0,
                                        1.0 - cell_probability.mine_probability / 100.0,
                                        0.0,
                                        0.5,
                                    )
                                },
                                _ => break 'no_overlay,
                            };
                            frame.fill_rectangle(
                                self.cell_position(x, y),
                                iced::Size::new(self.cell_size as f32, self.cell_size as f32),
                                overlay_color,
                            );
                        };
                    }
                }
            }
        });

        vec![geom]
    }
}
