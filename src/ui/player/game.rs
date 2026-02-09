use std::sync::Arc;

use crate::{
    base::board,
    ui::{board_area::BoardArea, skin},
};
use iced::widget::canvas;
use log::{debug, trace};

#[derive(Debug, Clone, Copy)]
pub enum BoardMessage {
    Left { x: usize, y: usize },
    Right { x: usize, y: usize },
    Chord { x: usize, y: usize, is_left: bool },
}

#[derive(Debug, Clone)]
pub enum GameMessage {
    Board(BoardMessage),
    FaceClicked,
    PressedPositionChanged,
    Resize {
        cell_size: u32,
        board_area: BoardArea,
        skin: Arc<skin::Skin>,
    },
    ChordModeChanged(board::ChordMode),
    ViewportChanged(iced::Rectangle),
    Continue,
    Replay,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum MouseState {
    #[default]
    Idle,
    LeftDown(Option<(usize, usize)>),
    RightDown,
    BothDown(Option<(usize, usize)>),
}

impl From<BoardMessage> for GameMessage {
    fn from(message: BoardMessage) -> Self {
        GameMessage::Board(message)
    }
}

pub struct Game {
    board: Box<dyn board::Board>,
    board_area: BoardArea,
    cell_size: u32,
    cache: canvas::Cache,
    skin: Arc<skin::Skin>,
    viewport: iced::Rectangle,
}

impl Game {
    pub fn new(board: Box<dyn board::Board>, board_area: BoardArea, cell_size: u32, skin: Arc<skin::Skin>) -> Self {
        Self {
            board,
            board_area,
            cell_size,
            cache: canvas::Cache::new(),
            skin,
            viewport: Default::default(),
        }
    }

    pub fn board(&self) -> &dyn board::Board {
        self.board.as_ref()
    }

    pub fn cell_size(&self) -> u32 {
        self.cell_size
    }

    fn cell_at(&self, pos: iced::Point) -> Option<(usize, usize)> {
        let x = ((pos.x - self.board_area.game_area.x) / self.cell_size as f32).floor() as isize;
        let y = ((pos.y - self.board_area.game_area.y) / self.cell_size as f32).floor() as isize;

        if x < 0 || y < 0 {
            return None;
        }

        let x = x as usize;
        let y = y as usize;

        if y < self.board.height() && x < self.board.width() {
            Some((x, y))
        } else {
            None
        }
    }

    fn cell_position(&self, x: usize, y: usize) -> iced::Point {
        iced::Point::new(
            x as f32 * self.cell_size as f32 + self.board_area.game_area.x,
            y as f32 * self.cell_size as f32 + self.board_area.game_area.y,
        )
    }

    pub fn update(&mut self, message: GameMessage) -> bool {
        trace!("GameMessage received: {:?}", message);
        match message {
            GameMessage::Board(board_msg) => {
                trace!("BoardMessage received: {:?}", board_msg);
                if self.board.state().is_end() {
                    debug!("Board is in end state ({:?}), ignoring input", self.board.state());
                    return false;
                }
                match board_msg {
                    BoardMessage::Left { x, y } => {
                        trace!("Left click at ({}, {})", x, y);
                        self.board.left_click(x, y);
                    },
                    BoardMessage::Right { x, y } => {
                        trace!("Right click at ({}, {})", x, y);
                        self.board.right_click(x, y);
                    },
                    BoardMessage::Chord { x, y, is_left } => {
                        trace!("Chord click at ({}, {}), is_left: {}", x, y, is_left);
                        self.board.chord_click(x, y, is_left);
                    },
                }
                self.cache.clear();
                return true;
            },
            GameMessage::FaceClicked => {
                debug!("Face clicked, resetting the board");
                self.board.reset();
                self.cache.clear();
                return true;
            },
            GameMessage::PressedPositionChanged => {
                trace!("PressedPositionChanged");
                self.cache.clear();
            },
            GameMessage::ChordModeChanged(mode) => {
                debug!("Changing chord mode to {:?}", mode);
                self.board.set_chord_mode(mode);
                self.cache.clear();
            },
            GameMessage::ViewportChanged(viewport) => {
                trace!("Viewport changed to {:?}", viewport);
                self.viewport = viewport;
                self.cache.clear();
            },
            GameMessage::Resize {
                cell_size,
                skin,
                board_area,
            } => {
                debug!("Resizing cell size to {}", cell_size);
                self.cell_size = cell_size;
                self.skin = skin;
                self.board_area = board_area;
                self.cache.clear();
            },
            GameMessage::Continue => {
                debug!("Continuing from lost state, resetting the board");
                self.board.resume();
                self.cache.clear();
            },
            GameMessage::Replay => {
                debug!("Replaying the current game, resetting the board");
                self.board.replay();
                self.cache.clear();
                return true;
            },
        }
        false
    }

    pub fn view(&self) -> iced::Element<'_, GameMessage> {
        canvas::Canvas::new(self)
            .width(self.board_area.canvas_area.width)
            .height(self.board_area.canvas_area.height)
            .into()
    }
}

impl canvas::Program<GameMessage> for Game {
    type State = MouseState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: iced::Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<GameMessage>> {
        match event {
            iced::Event::Mouse(mouse_event) => {
                let cursor_position = cursor.position_in(bounds);
                let position = cursor_position.and_then(|pos| {
                    self.cell_at(pos).or(self
                        .board_area
                        .face_area
                        .contains(pos)
                        .then_some((usize::MAX, usize::MAX)))
                });
                match mouse_event {
                    iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left) => {
                        if let Some((x, y)) = position {
                            trace!("Left button pressed at ({}, {})", x, y);
                        } else {
                            trace!("Left button pressed outside board and face area");
                        };
                        match state {
                            MouseState::Idle => {
                                *state = MouseState::LeftDown(position);
                                trace!("State changed from Idle to LeftDown");
                                trace!("Publishing PressedPositionChanged");
                                Some(canvas::Action::publish(GameMessage::PressedPositionChanged).and_capture())
                            },
                            MouseState::LeftDown { .. } => {
                                *state = MouseState::LeftDown(position);
                                trace!("State changed from LeftDown to LeftDown, maybe lost focus?");
                                trace!("Publishing PressedPositionChanged");
                                Some(canvas::Action::publish(GameMessage::PressedPositionChanged).and_capture())
                            },
                            MouseState::RightDown => {
                                *state = MouseState::BothDown(position);
                                trace!("State changed from RightDown to BothDown");
                                trace!("Publishing PressedPositionChanged");
                                Some(canvas::Action::publish(GameMessage::PressedPositionChanged).and_capture())
                            },
                            MouseState::BothDown { .. } => {
                                *state = MouseState::LeftDown(position);
                                trace!("State changed from BothDown to LeftDown, maybe lost focus?");
                                trace!("Publishing PressedPositionChanged");
                                Some(canvas::Action::publish(GameMessage::PressedPositionChanged).and_capture())
                            },
                        }
                    },
                    iced::mouse::Event::ButtonPressed(iced::mouse::Button::Right) => {
                        if let Some((x, y)) = position {
                            trace!("Right button pressed at ({}, {})", x, y);
                        } else {
                            trace!("Right button pressed outside board and face area");
                        };
                        match state {
                            MouseState::Idle => {
                                *state = MouseState::RightDown;
                                trace!("State changed from Idle to RightDown");
                                position.and_then(|(x, y)| {
                                    if x < self.board.width() && y < self.board.height() {
                                        trace!("Publishing Right click at ({}, {})", x, y);
                                        Some(
                                            canvas::Action::publish(GameMessage::Board(BoardMessage::Right { x, y }))
                                                .and_capture(),
                                        )
                                    } else {
                                        trace!("Right click at ({}, {}) is outside board area, ignoring", x, y);
                                        None
                                    }
                                })
                            },
                            MouseState::LeftDown { .. } => {
                                *state = MouseState::BothDown(position);
                                trace!("State changed from LeftDown to BothDown");
                                trace!("Publishing PressedPositionChanged");
                                Some(canvas::Action::publish(GameMessage::PressedPositionChanged).and_capture())
                            },
                            MouseState::RightDown => {
                                trace!("State changed from RightDown to RightDown, maybe lost focus?");
                                position.and_then(|(x, y)| {
                                    if x < self.board.width() && y < self.board.height() {
                                        trace!("Publishing Right click at ({}, {})", x, y);
                                        Some(
                                            canvas::Action::publish(GameMessage::Board(BoardMessage::Right { x, y }))
                                                .and_capture(),
                                        )
                                    } else {
                                        trace!("Right click at ({}, {}) is outside board area, ignoring", x, y);
                                        None
                                    }
                                })
                            },
                            MouseState::BothDown { .. } => {
                                *state = MouseState::RightDown;
                                trace!("State changed from BothDown to RightDown, maybe lost focus?");
                                position.and_then(|(x, y)| {
                                    if x < self.board.width() && y < self.board.height() {
                                        trace!("Publishing Right click at ({}, {})", x, y);
                                        Some(
                                            canvas::Action::publish(GameMessage::Board(BoardMessage::Right { x, y }))
                                                .and_capture(),
                                        )
                                    } else {
                                        trace!("Right click at ({}, {}) is outside board area, ignoring", x, y);
                                        None
                                    }
                                })
                            },
                        }
                    },
                    iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left) => {
                        if let Some((x, y)) = position {
                            trace!("Left button released at ({}, {})", x, y);
                        } else {
                            trace!("Left button released outside board and face area");
                        };
                        match state {
                            MouseState::LeftDown { .. } => {
                                *state = MouseState::Idle;
                                trace!("State changed from LeftDown to Idle");
                                position.and_then(|(x, y)| {
                                    if x < self.board.width() && y < self.board.height() {
                                        trace!("Publishing Left click at ({}, {})", x, y);
                                        Some(
                                            canvas::Action::publish(GameMessage::Board(BoardMessage::Left { x, y }))
                                                .and_capture(),
                                        )
                                    } else if (x == usize::MAX) && (y == usize::MAX) {
                                        trace!("Publishing FaceClicked");
                                        Some(canvas::Action::publish(GameMessage::FaceClicked).and_capture())
                                    } else {
                                        trace!(
                                            "Left click at ({}, {}) is outside board area and face area, ignoring",
                                            x, y
                                        );
                                        None
                                    }
                                })
                            },
                            MouseState::BothDown { .. } => {
                                *state = MouseState::RightDown;
                                trace!("State changed from BothDown to RightDown");
                                position.and_then(|(x, y)| {
                                    if x < self.board.width() && y < self.board.height() {
                                        trace!("Publishing Chord click at ({}, {})", x, y);
                                        Some(
                                            canvas::Action::publish(GameMessage::Board(BoardMessage::Chord {
                                                x,
                                                y,
                                                is_left: true,
                                            }))
                                            .and_capture(),
                                        )
                                    } else if (x == usize::MAX) && (y == usize::MAX) {
                                        trace!("Publishing FaceClicked");
                                        Some(canvas::Action::publish(GameMessage::FaceClicked).and_capture())
                                    } else {
                                        trace!(
                                            "Chord click at ({}, {}) is outside board area and face area, ignoring",
                                            x, y
                                        );
                                        None
                                    }
                                })
                            },
                            _ => {
                                trace!("Left button released but not pressed before, maybe lost focus?");
                                None
                            },
                        }
                    },
                    iced::mouse::Event::ButtonReleased(iced::mouse::Button::Right) => {
                        if let Some((x, y)) = position {
                            trace!("Right button released at ({}, {})", x, y);
                        } else {
                            trace!("Right button released outside board and face area");
                        };
                        match state {
                            MouseState::RightDown => {
                                *state = MouseState::Idle;
                                trace!("State changed from RightDown to Idle");
                                None
                            },
                            MouseState::BothDown { .. } => {
                                *state = MouseState::LeftDown(position);
                                trace!("State changed from BothDown to LeftDown");
                                position.and_then(|(x, y)| {
                                    if x < self.board.width() && y < self.board.height() {
                                        trace!("Publishing Chord click at ({}, {})", x, y);
                                        Some(
                                            canvas::Action::publish(GameMessage::Board(BoardMessage::Chord {
                                                x,
                                                y,
                                                is_left: false,
                                            }))
                                            .and_capture(),
                                        )
                                    } else {
                                        trace!("Chord click at ({}, {}) is outside board area, ignoring", x, y);
                                        None
                                    }
                                })
                            },
                            _ => {
                                trace!("Right button released but not pressed before, maybe lost focus?");
                                None
                            },
                        }
                    },
                    iced::mouse::Event::CursorMoved { .. } => match state {
                        MouseState::LeftDown(pos) => {
                            if *pos == position {
                                return None;
                            }
                            *state = MouseState::LeftDown(position);
                            trace!("Publishing PressedPositionChanged");
                            Some(canvas::Action::publish(GameMessage::PressedPositionChanged).and_capture())
                        },
                        MouseState::BothDown(pos) => {
                            if *pos == position {
                                return None;
                            }
                            *state = MouseState::BothDown(position);
                            trace!("Publishing PressedPositionChanged");
                            Some(canvas::Action::publish(GameMessage::PressedPositionChanged).and_capture())
                        },
                        _ => None,
                    },
                    _ => None,
                }
            },
            iced::Event::Keyboard(iced::keyboard::Event::KeyReleased {
                key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Space),
                ..
            }) => {
                trace!("Space key released, publishing FaceClicked");
                Some(canvas::Action::publish(GameMessage::FaceClicked).and_capture())
            },
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let geom = self.cache.draw(renderer, bounds.size(), |frame| {
            if self.board_area.top_area != iced::Rectangle::default() {
                if self.board_area.counter_area != iced::Rectangle::default() {
                    let mut negative = false;
                    let mut remaining = match self.board.state() {
                        board::BoardState::InProgress { flags, .. } | board::BoardState::Lost { flags, .. } => {
                            negative = flags > self.board.mines();
                            self.board.mines().abs_diff(flags)
                        },
                        board::BoardState::Won => 0,
                        _ => self.board.mines(),
                    };
                    remaining = remaining.clamp(0, if negative { 99 } else { 999 });
                    let mut digits = [remaining / 100, (remaining / 10) % 10, remaining % 10]
                        .map(|d| &self.skin.top_area.counter.digits[d]);
                    if negative {
                        match remaining {
                            1..=9 => {
                                digits[1] = &self.skin.top_area.counter.minus;
                            },
                            10..=99 => {
                                digits[2] = &self.skin.top_area.counter.minus;
                            },
                            _ => (),
                        }
                    }
                    for (rect, digit) in self.board_area.counter_digit_area.iter().zip(digits.iter()) {
                        frame.draw_image(*rect, *digit);
                    }
                }

                if self.board_area.face_area != iced::Rectangle::default() {
                    let face_img = match state {
                        MouseState::LeftDown(Some((usize::MAX, usize::MAX)))
                        | MouseState::BothDown(Some((usize::MAX, usize::MAX))) => &self.skin.top_area.face.pressed,
                        _ => match self.board.state() {
                            board::BoardState::Lost { .. } => &self.skin.top_area.face.lose,
                            board::BoardState::Won => &self.skin.top_area.face.win,
                            _ => &self.skin.top_area.face.normal,
                        },
                    };
                    frame.draw_image(self.board_area.face_area, face_img);
                }
            }

            // Calculate visible cell range for viewport culling
            let cell_size_f32 = self.cell_size as f32;

            // The viewport from scrollable is in scrollable content coordinates
            // bounds.position() gives us the canvas position within the scrollable content
            // We need to calculate which part of the canvas (board_area) is visible
            let canvas_x = bounds.x;
            let canvas_y = bounds.y;

            // Calculate the intersection of viewport and board area
            // Adjust board_area coordinates to scrollable content space
            let board_x_in_content = canvas_x + self.board_area.game_area.x;
            let board_y_in_content = canvas_y + self.board_area.game_area.y;
            let board_x_end = board_x_in_content + self.board_area.game_area.width;
            let board_y_end = board_y_in_content + self.board_area.game_area.height;

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
            let end_x = end_x_unclamped.min(self.board.width());
            let end_y = end_y_unclamped.min(self.board.height());

            trace!(
                "Viewport culling: canvas at ({}, {}), drawing cells from ({}, {}) to ({}, {}) out of board size {}x{}",
                canvas_x,
                canvas_y,
                start_x,
                start_y,
                end_x,
                end_y,
                self.board.width(),
                self.board.height()
            );

            for x in start_x..end_x {
                for y in start_y..end_y {
                    let draw_pressed = 'outer: {
                        // Board is in progress
                        if self.board.state().is_end() {
                            break 'outer false;
                        }
                        // Cell should be closed
                        if !matches!(
                            self.board.cell_state(x, y).unwrap_or(board::CellState::Opening(0)),
                            board::CellState::Closed
                        ) {
                            break 'outer false;
                        }
                        // Left mouse button is down over this cell
                        if matches!(
                            state,
                            MouseState::LeftDown(Some((sx, sy)))
                            | MouseState::BothDown(Some((sx, sy)))
                                if *sx == x && *sy == y
                        ) {
                            trace!("Pressed cell at ({}, {}), draw pressed", x, y);
                            break 'outer true;
                        }
                        match self.board.chord_mode() {
                            // Standard chord mode, don't need to check the center cell
                            board::ChordMode::Standard => {
                                if matches!(
                                    state,
                                    MouseState::BothDown(Some((sx, sy)))
                                        if sx.abs_diff(x) <=1 && sy.abs_diff(y) <=1
                                ) {
                                    trace!("Chording cell at ({}, {}), standard mode, draw pressed", x, y);
                                    break 'outer true;
                                }
                            },
                            // Left-click chord mode, need to check if the center cell is number
                            board::ChordMode::LeftClick => match state {
                                MouseState::LeftDown(Some((sx, sy))) | MouseState::BothDown(Some((sx, sy)))
                                    if sx.abs_diff(x) <= 1 && sy.abs_diff(y) <= 1 =>
                                {
                                    if matches!(
                                        self.board.cell_state(*sx, *sy).unwrap_or(board::CellState::Closed),
                                        board::CellState::Opening(1..)
                                    ) {
                                        trace!("Chording cell at ({}, {}), left-click mode, draw pressed", x, y);
                                        break 'outer true;
                                    }
                                },
                                _ => (),
                            },
                        }
                        false
                    };
                    let img = if draw_pressed {
                        &self.skin.cell.pressed
                    } else {
                        match self.board.cell_state(x, y) {
                            Some(board::CellState::Closed) => {
                                if matches!(self.board.cell_content(x, y), Some(board::CellContent::Mine)) {
                                    match self.board.state() {
                                        board::BoardState::Lost { .. } => &self.skin.cell.mine.unflagged,
                                        board::BoardState::Won => &self.skin.cell.mine.flagged,
                                        _ => &self.skin.cell.closed,
                                    }
                                } else {
                                    &self.skin.cell.closed
                                }
                            },
                            Some(board::CellState::Flagged) => {
                                if matches!(self.board.state(), board::BoardState::Lost { .. })
                                    && !matches!(self.board.cell_content(x, y), Some(board::CellContent::Mine))
                                {
                                    &self.skin.cell.mine.wrong
                                } else {
                                    &self.skin.cell.mine.flagged
                                }
                            },
                            Some(board::CellState::Blasted) => &self.skin.cell.mine.blasted,
                            Some(board::CellState::Opening(n)) => match n {
                                1..=8 => &self.skin.cell.numbers[(n - 1) as usize],
                                _ => &self.skin.cell.opening,
                            },
                            _ => &self.skin.cell.opening,
                        }
                    };

                    frame.draw_image(
                        iced::Rectangle::new(
                            self.cell_position(x, y),
                            iced::Size::new(self.cell_size as f32, self.cell_size as f32),
                        ),
                        img,
                    );
                }
            }
        });

        vec![geom]
    }
}
