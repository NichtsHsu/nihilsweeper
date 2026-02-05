use crate::{base::board, config::GlobalConfig, ui::skin};
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
    Resize(u32, Box<skin::Skin>),
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
    game_area: iced::Rectangle,
    top_area: iced::Rectangle,
    board_area: iced::Rectangle,
    counter_area: iced::Rectangle,
    counter_digit_area: [iced::Rectangle; 3],
    face_area: iced::Rectangle,
    borders: Vec<iced::Rectangle>,
    light_paths: Vec<canvas::Path>,
    shadow_paths: Vec<canvas::Path>,
    cell_size: u32,
    foreground_cache: canvas::Cache,
    background_cache: canvas::Cache,
    skin: skin::Skin,
    viewport: iced::Rectangle,
}

impl Game {
    pub fn new(board: Box<dyn board::Board>, cell_size: u32, skin: skin::Skin) -> Self {
        let mut this = Self {
            board,
            game_area: iced::Rectangle::default(),
            top_area: iced::Rectangle::default(),
            board_area: iced::Rectangle::default(),
            counter_area: iced::Rectangle::default(),
            counter_digit_area: [iced::Rectangle::default(); 3],
            face_area: iced::Rectangle::default(),
            borders: Vec::new(),
            light_paths: Vec::new(),
            shadow_paths: Vec::new(),
            cell_size,
            foreground_cache: canvas::Cache::new(),
            background_cache: canvas::Cache::new(),
            skin,
            viewport: Default::default(),
        };
        this.calculate_areas();
        this
    }

    fn calculate_areas(&mut self) {
        let mut top_area = iced::Rectangle {
            x: self.skin.border.width,
            y: self.skin.border.width,
            width: self.board.width() as f32 * self.cell_size as f32,
            height: self.skin.top_area.height,
        };

        let counter_offset = ((self.skin.top_area.height - self.skin.top_area.counter.height) / 2.0).floor();
        let counter_border_width = ((self.skin.top_area.counter.height
            - self.skin.top_area.counter.content_height
            - self.skin.top_area.counter.content_gap * 2.0)
            / 2.0)
            .floor();
        let counter_right_top;
        let mut counter_area = iced::Rectangle {
            x: top_area.x + counter_offset + counter_border_width,
            y: top_area.y + counter_offset + counter_border_width,
            width: self.skin.top_area.counter.content_width * 3.0 + self.skin.top_area.counter.content_gap * 6.0,
            height: self.skin.top_area.counter.content_height + self.skin.top_area.counter.content_gap * 2.0,
        };
        let counter_digit_area;
        if (counter_area.x + counter_area.width + counter_border_width + counter_offset) > (top_area.x + top_area.width)
        {
            debug!("Not enough space for counter, skipping");
            trace!(
                "counter_area.x = {}, counter_area.width = {}, counter_border_width = {}, counter_offset = {}, \
                 top_area.x = {}, top_area.width = {}",
                counter_area.x, counter_area.width, counter_border_width, counter_offset, top_area.x, top_area.width
            );
            counter_area = iced::Rectangle::default();
            counter_digit_area = [iced::Rectangle::default(); 3];
            counter_right_top = 0.0;
        } else {
            counter_digit_area = [
                iced::Rectangle {
                    x: counter_area.x + self.skin.top_area.counter.content_gap,
                    y: counter_area.y + self.skin.top_area.counter.content_gap,
                    width: self.skin.top_area.counter.content_width,
                    height: self.skin.top_area.counter.content_height,
                },
                iced::Rectangle {
                    x: counter_area.x
                        + self.skin.top_area.counter.content_width
                        + self.skin.top_area.counter.content_gap * 3.0,
                    y: counter_area.y + self.skin.top_area.counter.content_gap,
                    width: self.skin.top_area.counter.content_width,
                    height: self.skin.top_area.counter.content_height,
                },
                iced::Rectangle {
                    x: counter_area.x
                        + self.skin.top_area.counter.content_width * 2.0
                        + self.skin.top_area.counter.content_gap * 5.0,
                    y: counter_area.y + self.skin.top_area.counter.content_gap,
                    width: self.skin.top_area.counter.content_width,
                    height: self.skin.top_area.counter.content_height,
                },
            ];
            counter_right_top = counter_area.x + counter_area.width + counter_border_width * 2.0;
        }

        let face_offset = ((self.skin.top_area.height - self.skin.top_area.face.size) / 2.0).floor();
        let face_area;
        if (counter_right_top + self.skin.top_area.face.size + face_offset * 2.0) > top_area.width {
            debug!("Not enough space for face, skipping");
            face_area = iced::Rectangle::default();
        } else if counter_right_top + self.skin.top_area.face.size / 2.0 + face_offset
            < top_area.x + top_area.width / 2.0
        {
            debug!("Placing face in the center");
            face_area = iced::Rectangle {
                x: top_area.x + (top_area.width - self.skin.top_area.face.size) / 2.0,
                y: top_area.y + face_offset,
                width: self.skin.top_area.face.size,
                height: self.skin.top_area.face.size,
            };
        } else {
            debug!("Placing face to the right of counter");
            face_area = iced::Rectangle {
                x: counter_right_top + face_offset,
                y: top_area.y + face_offset,
                width: self.skin.top_area.face.size,
                height: self.skin.top_area.face.size,
            };
        }

        let game_area_offset = if counter_area == iced::Rectangle::default() && face_area == iced::Rectangle::default()
        {
            debug!("No top area, adjusting game area accordingly");
            top_area = iced::Rectangle::default();
            self.skin.border.width
        } else {
            top_area.height + self.skin.border.width * 2.0
        };
        let board_area = iced::Rectangle {
            x: self.skin.border.width,
            y: game_area_offset,
            width: self.board.width() as f32 * self.cell_size as f32,
            height: self.board.height() as f32 * self.cell_size as f32,
        };

        let game_area = iced::Rectangle {
            x: 0.0,
            y: 0.0,
            width: self.board.width() as f32 * self.cell_size as f32 + self.skin.border.width * 2.0,
            height: board_area.y + board_area.height + self.skin.border.width,
        };

        let mut borders = vec![
            iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: self.skin.border.width,
                height: game_area.height,
            },
            iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: game_area.width,
                height: self.skin.border.width,
            },
            iced::Rectangle {
                x: game_area.x + game_area.width - self.skin.border.width,
                y: 0.0,
                width: self.skin.border.width,
                height: game_area.height,
            },
            iced::Rectangle {
                x: 0.0,
                y: game_area.y + game_area.height - self.skin.border.width,
                width: game_area.width,
                height: self.skin.border.width,
            },
        ];

        let light_shadow_width = self.skin.border.width / 4.0;
        let main_light_path = canvas::Path::new(|p| {
            p.move_to(game_area.position());
            p.line_to(iced::Point::new(game_area.x + game_area.width, game_area.y));
            p.line_to(iced::Point::new(
                game_area.x + game_area.width - light_shadow_width,
                game_area.y + light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                game_area.x + light_shadow_width,
                game_area.y + light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                game_area.x + light_shadow_width,
                game_area.y + game_area.height - light_shadow_width,
            ));
            p.line_to(iced::Point::new(game_area.x, game_area.y + game_area.height));
            p.close();
        });
        let main_shadow_path = canvas::Path::new(|p| {
            p.move_to(iced::Point::new(game_area.x + game_area.width, game_area.y));
            p.line_to(iced::Point::new(
                game_area.x + game_area.width,
                game_area.y + game_area.height,
            ));
            p.line_to(iced::Point::new(game_area.x, game_area.y + game_area.height));
            p.line_to(iced::Point::new(
                game_area.x + light_shadow_width,
                game_area.y + game_area.height - light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                game_area.x + game_area.width - light_shadow_width,
                game_area.y + game_area.height - light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                game_area.x + game_area.width - light_shadow_width,
                game_area.y + light_shadow_width,
            ));
            p.close();
        });
        let board_light_path = canvas::Path::new(|p| {
            p.move_to(iced::Point::new(board_area.x + board_area.width, board_area.y));
            p.line_to(iced::Point::new(
                board_area.x + board_area.width,
                board_area.y + board_area.height,
            ));
            p.line_to(iced::Point::new(board_area.x, board_area.y + board_area.height));
            p.line_to(iced::Point::new(
                board_area.x - light_shadow_width,
                board_area.y + board_area.height + light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                board_area.x + board_area.width + light_shadow_width,
                board_area.y + board_area.height + light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                board_area.x + board_area.width + light_shadow_width,
                board_area.y - light_shadow_width,
            ));
            p.close();
        });
        let board_shadow_path = canvas::Path::new(|p| {
            p.move_to(board_area.position());
            p.line_to(iced::Point::new(board_area.x + board_area.width, board_area.y));
            p.line_to(iced::Point::new(
                board_area.x + board_area.width + light_shadow_width,
                board_area.y - light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                board_area.x - light_shadow_width,
                board_area.y - light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                board_area.x - light_shadow_width,
                board_area.y + board_area.height + light_shadow_width,
            ));
            p.line_to(iced::Point::new(board_area.x, board_area.y + board_area.height));
            p.close();
        });

        let mut light_paths = vec![main_light_path, board_light_path];
        let mut shadow_paths = vec![main_shadow_path, board_shadow_path];

        if top_area != iced::Rectangle::default() {
            borders.push(iced::Rectangle {
                x: 0.0,
                y: top_area.y + top_area.height,
                width: game_area.width,
                height: self.skin.border.width,
            });
            let top_light_path = canvas::Path::new(|p| {
                p.move_to(iced::Point::new(top_area.x + top_area.width, top_area.y));
                p.line_to(iced::Point::new(
                    top_area.x + top_area.width,
                    top_area.y + top_area.height,
                ));
                p.line_to(iced::Point::new(top_area.x, top_area.y + top_area.height));
                p.line_to(iced::Point::new(
                    top_area.x - light_shadow_width,
                    top_area.y + top_area.height + light_shadow_width,
                ));
                p.line_to(iced::Point::new(
                    top_area.x + top_area.width + light_shadow_width,
                    top_area.y + top_area.height + light_shadow_width,
                ));
                p.line_to(iced::Point::new(
                    top_area.x + top_area.width + light_shadow_width,
                    top_area.y - light_shadow_width,
                ));
                p.close();
            });
            let top_shadow_path = canvas::Path::new(|p| {
                p.move_to(top_area.position());
                p.line_to(iced::Point::new(top_area.x + top_area.width, top_area.y));
                p.line_to(iced::Point::new(
                    top_area.x + top_area.width + light_shadow_width,
                    top_area.y - light_shadow_width,
                ));
                p.line_to(iced::Point::new(
                    top_area.x - light_shadow_width,
                    top_area.y - light_shadow_width,
                ));
                p.line_to(iced::Point::new(
                    top_area.x - light_shadow_width,
                    top_area.y + top_area.height + light_shadow_width,
                ));
                p.line_to(iced::Point::new(top_area.x, top_area.y + top_area.height));
                p.close();
            });
            light_paths.push(top_light_path);
            shadow_paths.push(top_shadow_path);
        }

        if counter_area != iced::Rectangle::default() && counter_border_width > 0.0 {
            let counter_light_path = canvas::Path::new(|p| {
                p.move_to(iced::Point::new(counter_area.x + counter_area.width, counter_area.y));
                p.line_to(iced::Point::new(
                    counter_area.x + counter_area.width,
                    counter_area.y + counter_area.height,
                ));
                p.line_to(iced::Point::new(counter_area.x, counter_area.y + counter_area.height));
                p.line_to(iced::Point::new(
                    counter_area.x - counter_border_width,
                    counter_area.y + counter_area.height + counter_border_width,
                ));
                p.line_to(iced::Point::new(
                    counter_area.x + counter_area.width + counter_border_width,
                    counter_area.y + counter_area.height + counter_border_width,
                ));
                p.line_to(iced::Point::new(
                    counter_area.x + counter_area.width + counter_border_width,
                    counter_area.y - counter_border_width,
                ));
                p.close();
            });
            let counter_shadow_path = canvas::Path::new(|p| {
                p.move_to(counter_area.position());
                p.line_to(iced::Point::new(counter_area.x + counter_area.width, counter_area.y));
                p.line_to(iced::Point::new(
                    counter_area.x + counter_area.width + counter_border_width,
                    counter_area.y - counter_border_width,
                ));
                p.line_to(iced::Point::new(
                    counter_area.x - counter_border_width,
                    counter_area.y - counter_border_width,
                ));
                p.line_to(iced::Point::new(
                    counter_area.x - counter_border_width,
                    counter_area.y + counter_area.height + counter_border_width,
                ));
                p.line_to(iced::Point::new(counter_area.x, counter_area.y + counter_area.height));
                p.close();
            });
            light_paths.push(counter_light_path);
            shadow_paths.push(counter_shadow_path);
        }

        self.game_area = game_area;
        self.top_area = top_area;
        self.board_area = board_area;
        self.counter_area = counter_area;
        self.counter_digit_area = counter_digit_area;
        self.face_area = face_area;
        self.borders = borders;
        self.light_paths = light_paths;
        self.shadow_paths = shadow_paths;
    }

    pub fn board(&self) -> &dyn board::Board {
        self.board.as_ref()
    }

    pub fn board_area(&self) -> iced::Rectangle {
        self.board_area
    }

    pub fn game_area(&self) -> iced::Rectangle {
        self.game_area
    }

    pub fn cell_size(&self) -> u32 {
        self.cell_size
    }

    fn cell_at(&self, pos: iced::Point) -> Option<(usize, usize)> {
        let x = ((pos.x - self.board_area.x) / self.cell_size as f32).floor() as isize;
        let y = ((pos.y - self.board_area.y) / self.cell_size as f32).floor() as isize;

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
            x as f32 * self.cell_size as f32 + self.board_area.x,
            y as f32 * self.cell_size as f32 + self.board_area.y,
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
                self.foreground_cache.clear();
                return true;
            },
            GameMessage::FaceClicked => {
                debug!("Face clicked, resetting the board");
                self.board.reset();
                self.foreground_cache.clear();
                return true;
            },
            GameMessage::PressedPositionChanged => {
                trace!("PressedPositionChanged");
                self.foreground_cache.clear();
            },
            GameMessage::ChordModeChanged(mode) => {
                debug!("Changing chord mode to {:?}", mode);
                self.board.set_chord_mode(mode);
                self.foreground_cache.clear();
            },
            GameMessage::ViewportChanged(viewport) => {
                trace!("Viewport changed to {:?}", viewport);
                self.viewport = viewport;
                self.foreground_cache.clear();
            },
            GameMessage::Resize(cell_size, skin) => {
                debug!("Resizing cell size to {}", cell_size);
                self.cell_size = cell_size;
                self.skin = *skin;
                self.calculate_areas();
                self.foreground_cache.clear();
                self.background_cache.clear();
            },
            GameMessage::Continue => {
                debug!("Continuing from lost state, resetting the board");
                self.board.resume();
                self.foreground_cache.clear();
            },
            GameMessage::Replay => {
                debug!("Replaying the current game, resetting the board");
                self.board.replay();
                self.foreground_cache.clear();
                return true;
            },
        }
        false
    }

    pub fn view(&self) -> iced::Element<'_, GameMessage> {
        canvas::Canvas::new(self)
            .width(self.game_area.width)
            .height(self.game_area.height)
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
                    self.cell_at(pos)
                        .or(self.face_area.contains(pos).then_some((usize::MAX, usize::MAX)))
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
        let background_geom = self.background_cache.draw(renderer, bounds.size(), |frame| {
            let background = canvas::Path::rectangle(iced::Point::ORIGIN, frame.size());
            frame.fill(&background, self.skin.background_color);
            if self.top_area != iced::Rectangle::default() {
                let top_area_background = canvas::Path::rectangle(self.top_area.position(), self.top_area.size());
                frame.fill(&top_area_background, self.skin.top_area.background_color);
                if self.counter_area != iced::Rectangle::default() {
                    let counter_background =
                        canvas::Path::rectangle(self.counter_area.position(), self.counter_area.size());
                    frame.fill(&counter_background, self.skin.top_area.counter.background_color);
                }
            }
            for border in &self.borders {
                frame.fill_rectangle(border.position(), border.size(), self.skin.border.color);
            }
            for light_path in &self.light_paths {
                frame.fill(light_path, self.skin.highlight_color);
            }
            for shadow_path in &self.shadow_paths {
                frame.fill(shadow_path, self.skin.shadow_color);
            }
        });

        let foreground_geom = self.foreground_cache.draw(renderer, bounds.size(), |frame| {
            if self.top_area != iced::Rectangle::default() {
                if self.counter_area != iced::Rectangle::default() {
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
                    for (rect, digit) in self.counter_digit_area.iter().zip(digits.iter()) {
                        frame.draw_image(*rect, *digit);
                    }
                }

                if self.face_area != iced::Rectangle::default() {
                    let face_img = match state {
                        MouseState::LeftDown(Some((usize::MAX, usize::MAX)))
                        | MouseState::BothDown(Some((usize::MAX, usize::MAX))) => &self.skin.top_area.face.pressed,
                        _ => match self.board.state() {
                            board::BoardState::Lost { .. } => &self.skin.top_area.face.lose,
                            board::BoardState::Won => &self.skin.top_area.face.win,
                            _ => &self.skin.top_area.face.normal,
                        },
                    };
                    frame.draw_image(self.face_area, face_img);
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

                    // 'no_overlay: {
                    //     if let Some(solver) = &self.solver_result
                    //         && let Some(cell_safety) = solver.get(x, y)
                    //     {
                    //         let overlay_color = match cell_safety {
                    //             crate::engine::solver::CellSafety::Safe =>
                    // iced::Color::from_rgba(0.0, 1.0, 0.0, 0.5),
                    // crate::engine::solver::CellSafety::Mine => iced::Color::from_rgba(1.0, 0.0,
                    // 0.0, 0.5),
                    // crate::engine::solver::CellSafety::Probability(cell_probability) => {
                    //                 iced::Color::from_rgba(
                    //                     cell_probability.mine_probability / 100.0,
                    //                     1.0 - cell_probability.mine_probability / 100.0,
                    //                     0.0,
                    //                     0.5,
                    //                 )
                    //             },
                    //             _ => break 'no_overlay,
                    //         };
                    //         trace!("Drawing solver overlay at ({}, {}) with color {:?}", x, y,
                    // overlay_color);         frame.fill_rectangle(
                    //             self.cell_position(x, y),
                    //             iced::Size::new(self.cell_size as f32, self.cell_size as f32),
                    //             overlay_color,
                    //         );
                    //     };
                    // }
                }
            }
        });

        vec![background_geom, foreground_geom]
    }
}
