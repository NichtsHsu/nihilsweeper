use crate::{
    base::board,
    engine::solver::{self, BoardSafety, Solver},
};
use iced::widget::canvas;
use log::{error, trace};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum SolverOverlayMessage {
    SolverCompleted(solver::error::Result<BoardSafety>),
    Resize {
        cell_size: u32,
        game_area: iced::Rectangle,
        board_area: iced::Rectangle,
    },
    SetEnabled(bool),
    SetAdmitFlags(bool),
    SetLightSkin(bool),
}

pub struct SolverOverlay {
    enabled: bool,
    solver: Arc<dyn Solver>,
    solver_result: Option<BoardSafety>,
    solver_admit_flags: bool,
    game_area: iced::Rectangle,
    board_area: iced::Rectangle,
    viewport: iced::Rectangle,
    cell_size: u32,
    cache: canvas::Cache,
    light_skin: bool,
}

impl SolverOverlay {
    pub fn new<T: Solver + 'static>(
        solver: T,
        game_area: iced::Rectangle,
        board_area: iced::Rectangle,
        viewport: iced::Rectangle,
        cell_size: u32,
    ) -> Self {
        SolverOverlay {
            enabled: false,
            solver: Arc::new(solver),
            solver_result: None,
            solver_admit_flags: false,
            game_area,
            board_area,
            viewport,
            cell_size,
            cache: canvas::Cache::new(),
            light_skin: true,
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

    pub fn update_solver(&mut self, board: &dyn board::Board) -> iced::Task<SolverOverlayMessage> {
        if !self.enabled {
            return iced::Task::none();
        }
        let solver = Arc::clone(&self.solver);
        let mines = board.mines();
        let cell_states = board.cell_states().clone();
        let admit_flags = self.solver_admit_flags;

        iced::Task::perform(
            async move {
                let board_safety = BoardSafety::new(&cell_states, mines, admit_flags);
                solver.calculate(board_safety)
            },
            SolverOverlayMessage::SolverCompleted,
        )
    }

    pub fn clear_solver(&mut self) {
        self.solver_result = None;
        self.cache.clear();
    }

    pub fn update(&mut self, message: SolverOverlayMessage) {
        match message {
            SolverOverlayMessage::SolverCompleted(result) => {
                debug!("Solver completed, updating overlay");
                self.solver_result = result.inspect_err(|e| error!("Solver error: {:?}", e)).ok();
                self.cache.clear();
            },
            SolverOverlayMessage::Resize {
                cell_size,
                game_area,
                board_area,
            } => {
                self.cell_size = cell_size;
                self.game_area = game_area;
                self.board_area = board_area;
                self.cache.clear();
            },
            SolverOverlayMessage::SetEnabled(enabled) => {
                self.enabled = enabled;
                if !enabled {
                    self.clear_solver();
                }
            },
            SolverOverlayMessage::SetAdmitFlags(admit_flags) => {
                self.solver_admit_flags = admit_flags;
                self.clear_solver();
            },
            SolverOverlayMessage::SetLightSkin(light_skin) => {
                self.light_skin = light_skin;
                self.cache.clear();
            },
        }
    }

    pub fn view(&self) -> iced::Element<'_, SolverOverlayMessage> {
        canvas::Canvas::new(self)
            .width(self.game_area.width)
            .height(self.game_area.height)
            .into()
    }
}

impl canvas::Program<SolverOverlayMessage> for SolverOverlay {
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

        let Some(board) = &self.solver_result else {
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
                    let Some(solver) = &self.solver_result else {
                        continue;
                    };
                    let Some(cell_safety) = solver.get(x, y) else {
                        continue;
                    };

                    match cell_safety {
                        crate::engine::solver::CellSafety::Safe => {
                            let overlay_color = iced::Color::from_rgba(0.0, 1.0, 0.0, 0.5);
                            frame.fill_rectangle(
                                self.cell_position(x, y),
                                iced::Size::new(self.cell_size as f32, self.cell_size as f32),
                                overlay_color,
                            );
                        },
                        crate::engine::solver::CellSafety::Mine => {
                            let overlay_color = iced::Color::from_rgba(1.0, 0.0, 0.0, 0.5);
                            frame.fill_rectangle(
                                self.cell_position(x, y),
                                iced::Size::new(self.cell_size as f32, self.cell_size as f32),
                                overlay_color,
                            );
                        },
                        crate::engine::solver::CellSafety::Probability(cell_probability) => {
                            // Calculate color based on mine probability (0.0-1.0 range)
                            let text_color = if cell_probability.frontier {
                                if self.light_skin {
                                    iced::Color::from_rgb(
                                        cell_probability.mine_probability * 0.65,
                                        (1.0 - cell_probability.mine_probability) * 0.65,
                                        0.0,
                                    )
                                } else {
                                    iced::Color::from_rgb(
                                        cell_probability.mine_probability,
                                        1.0 - cell_probability.mine_probability,
                                        0.0,
                                    )
                                }
                            } else {
                                iced::Color::from_rgb(0.5, 0.5, 0.5)
                            };

                            // Scale probability to 0.0-100.0 and format with up to 3 total digits
                            let probability_percent = cell_probability.mine_probability * 100.0;

                            // Format to show meaningful digits
                            let probability_text = if probability_percent >= 10.0 {
                                // For values >= 10, show 1 decimal place (e.g., 12.3, 46.0)
                                format!("{:.1}", probability_percent)
                            } else {
                                // For values < 10, show 2 decimal places (e.g., 1.23, 0.01)
                                format!("{:.2}", probability_percent)
                            };

                            // Draw text centered in the cell
                            let cell_pos = self.cell_position(x, y);
                            let text_size = self.cell_size as f32 * 0.4; // Adjust text size relative to cell

                            // Center the text in the cell by adjusting position
                            // Text is drawn from baseline, so we offset it
                            let text_position = iced::Point::new(
                                cell_pos.x + self.cell_size as f32 * 0.5,
                                cell_pos.y + self.cell_size as f32 * 0.5,
                            );

                            frame.fill_text(canvas::Text {
                                content: probability_text,
                                position: text_position,
                                color: text_color,
                                size: text_size.into(),
                                font: iced::Font {
                                    weight: iced::font::Weight::Bold,
                                    ..Default::default()
                                },
                                max_width: self.cell_size as f32,
                                line_height: iced::widget::text::LineHeight::Relative(1.0),
                                align_x: iced::widget::text::Alignment::Center,
                                align_y: iced::alignment::Vertical::Center,
                                ..Default::default()
                            });
                        },
                        _ => continue,
                    }
                }
            }
        });

        vec![geom]
    }
}
