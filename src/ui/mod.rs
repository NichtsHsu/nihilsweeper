pub mod game;
pub mod skin;

use crate::{
    base::board::{self, Board},
    config::GlobalConfig,
};
use iced::{Function, Task};
use log::{debug, error, trace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputType {
    Width = 0,
    Height = 1,
    Mines = 2,
}

#[derive(Debug, Clone)]
pub enum MainWindowMessage {
    Game(game::GameMessage),
    TextInputChanged(TextInputType, String),
    ChordModeToggled(bool),
    Scrolled(iced::widget::scrollable::Viewport),
}

impl From<game::GameMessage> for MainWindowMessage {
    fn from(message: game::GameMessage) -> Self {
        MainWindowMessage::Game(message)
    }
}

pub struct MainWindow {
    config: GlobalConfig,
    skin_manager: Option<skin::SkinManager>,
    skin: Option<skin::Skin>,
    game: Option<game::Game>,
    text_input_states: [String; 3],
}

impl MainWindow {
    const TEXT_INPUT_LIMITS: [usize; 3] = [999, 999, 999999];

    pub fn new() -> (Self, Task<MainWindowMessage>) {
        let config = GlobalConfig {
            chord_mode: board::ChordMode::LeftClick,
            skin: "WoM Light".to_string(),
            cell_size: 24,
            board: [30, 16, 99],
        };
        let board = Box::new(board::StandardBoard::new(
            config.board[0],
            config.board[1],
            config.board[2],
            config.chord_mode,
        ));
        let skin_manager = crate::utils::resource_path("skin")
            .inspect_err(|e| error!("Failed to get skin resource path: {}", e))
            .and_then(|path| {
                skin::SkinManager::new(path).inspect_err(|e| error!("Failed to initialize SkinManager: {}", e))
            })
            .ok();
        let skin = skin_manager
            .as_ref()
            .and_then(|manager| {
                manager.skins().get(&config.skin).or_else(|| {
                    error!("Skin '{}' not found.", config.skin);
                    None
                })
            })
            .and_then(|builder| {
                builder
                    .build(config.cell_size)
                    .inspect_err(|e| error!("Failed to build skin '{}': {}", config.skin, e))
                    .ok()
            });
        let game = skin.clone().and_then(|skin| {
            game::Game::new(board, &config, skin)
                .inspect_err(|e| error!("Failed to initialize game: {}", e))
                .ok()
        });
        let text_input_states = [
            config.board[0].to_string(),
            config.board[1].to_string(),
            config.board[2].to_string(),
        ];
        (
            Self {
                config,
                skin_manager,
                skin,
                game,
                text_input_states,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: MainWindowMessage) -> iced::Task<MainWindowMessage> {
        match message {
            MainWindowMessage::Game(game_msg) => {
                let is_face_clicked = matches!(game_msg, game::GameMessage::FaceClicked);

                if is_face_clicked {
                    let current_board = self
                        .game
                        .as_ref()
                        .map(|game| [game.board().width(), game.board().height(), game.board().mines()]);
                    if Some(self.config.board) != current_board {
                        debug!("Board configuration changed, recreating the board");
                        let new_board = Box::new(board::StandardBoard::new(
                            self.config.board[0],
                            self.config.board[1],
                            self.config.board[2],
                            self.config.chord_mode,
                        ));

                        self.game = self
                            .skin
                            .clone()
                            .and_then(|skin| {
                                game::Game::new(new_board, &self.config, skin)
                                    .inspect_err(|e| error!("Failed to initialize game: {}", e))
                                    .ok()
                            })
                            .inspect(|game| {
                                self.config.board = [game.board().width(), game.board().height(), game.board().mines()];
                                self.text_input_states = [
                                    self.config.board[0].to_string(),
                                    self.config.board[1].to_string(),
                                    self.config.board[2].to_string(),
                                ];
                            });
                        return Task::none();
                    }
                }
                if let Some(game) = &mut self.game {
                    game.update(game_msg);
                    if is_face_clicked {
                        self.config.board = [game.board().width(), game.board().height(), game.board().mines()];
                        self.text_input_states = [
                            self.config.board[0].to_string(),
                            self.config.board[1].to_string(),
                            self.config.board[2].to_string(),
                        ];
                    }
                }
            },
            MainWindowMessage::TextInputChanged(input_type, value) => {
                if value.is_empty() {
                    self.text_input_states[input_type as usize] = value;
                    self.config.board[input_type as usize] = if let Some(game) = &self.game {
                        match input_type {
                            TextInputType::Width => game.board().width(),
                            TextInputType::Height => game.board().height(),
                            TextInputType::Mines => game.board().mines(),
                        }
                    } else {
                        40
                    };
                    return Task::none();
                }
                if let Ok(mut num) = value.parse::<usize>() {
                    num = num.clamp(1, MainWindow::TEXT_INPUT_LIMITS[input_type as usize]);
                    self.text_input_states[input_type as usize] = num.to_string();
                    self.config.board[input_type as usize] = num;
                    trace!(
                        "Text input changed: {:?} = {}, updated board config to {:?}",
                        input_type, num, self.config.board
                    );
                }
            },
            MainWindowMessage::ChordModeToggled(enabled) => {
                self.config.chord_mode = if enabled {
                    board::ChordMode::LeftClick
                } else {
                    board::ChordMode::Standard
                };
                if let Some(game) = &mut self.game {
                    game.update(game::GameMessage::ChordModeChanged(self.config.chord_mode));
                }
                trace!("Chord mode toggled: {:?}", self.config.chord_mode);
            },
            MainWindowMessage::Scrolled(viewport) => {
                if let Some(game) = &mut self.game {
                    let absolute_offset = viewport.absolute_offset();
                    let bounds = viewport.bounds();
                    let viewport_rect = iced::Rectangle {
                        x: absolute_offset.x,
                        y: absolute_offset.y,
                        width: bounds.width,
                        height: bounds.height,
                    };
                    trace!("Scroll event: viewport = {:?}", viewport_rect);
                    game.update(game::GameMessage::ViewportChanged(viewport_rect));
                }
            },
        };
        Task::none()
    }

    pub fn view(&self) -> iced::Element<'_, MainWindowMessage> {
        let (base_color, lower_color, upper_color) = self.skin.as_ref().map_or(
            (
                iced::Color::from_rgb8(128, 128, 128),
                iced::Color::WHITE,
                iced::Color::BLACK,
            ),
            |skin| (skin.background_color, skin.highlight_color, skin.shadow_color),
        );
        let text_color = lower_color.inverse();
        let text_input_style = iced::widget::text_input::Style {
            background: iced::Background::Color(lower_color),
            border: iced::Border {
                color: upper_color,
                width: 2.0,
                radius: iced::border::radius(2.0),
            },
            icon: iced::Color::TRANSPARENT,
            placeholder: text_color.scale_alpha(0.5),
            value: text_color,
            selection: base_color,
        };
        let button_style = iced::widget::button::Style {
            background: Some(iced::Background::Color(lower_color)),
            border: iced::Border {
                color: upper_color,
                width: 2.0,
                radius: iced::border::radius(4.0),
            },
            text_color,
            ..Default::default()
        };
        let check_box_style = iced::widget::checkbox::Style {
            background: iced::Background::Color(iced::Color::TRANSPARENT),
            border: iced::Border {
                color: upper_color,
                width: 2.0,
                radius: iced::border::radius(4.0),
            },
            icon_color: text_color,
            text_color: Some(text_color),
        };

        let board_control = iced::widget::container(
            iced::widget::column![
                iced::widget::center_x(iced::widget::Text::new("Board").size(16).color(text_color)),
                iced::widget::center_x(
                    iced::widget::TextInput::new("width", &self.text_input_states[TextInputType::Width as usize])
                        .on_input(MainWindowMessage::TextInputChanged.with(TextInputType::Width))
                        .style(move |_, _| text_input_style)
                ),
                iced::widget::center_x(
                    iced::widget::TextInput::new("height", &self.text_input_states[TextInputType::Height as usize])
                        .on_input(MainWindowMessage::TextInputChanged.with(TextInputType::Height))
                        .style(move |_, _| text_input_style)
                ),
                iced::widget::center_x(
                    iced::widget::TextInput::new("mines", &self.text_input_states[TextInputType::Mines as usize])
                        .on_input(MainWindowMessage::TextInputChanged.with(TextInputType::Mines))
                        .style(move |_, _| text_input_style)
                ),
                iced::widget::center_x(
                    iced::widget::button("New Game")
                        .style(move |_, _| button_style)
                        .on_press(MainWindowMessage::Game(game::GameMessage::FaceClicked))
                )
            ]
            .spacing(4)
            .padding(6),
        )
        .style(move |_| iced::widget::container::Style {
            border: iced::Border {
                color: upper_color,
                width: 2.0,
                radius: iced::border::radius(4.0),
            },
            ..Default::default()
        });
        let control_panel = iced::widget::container(
            iced::widget::column![
                iced::widget::center_x(iced::widget::Text::new("Control Panel").size(20).color(text_color)),
                iced::widget::center_x(board_control),
                iced::widget::center_x(
                    iced::widget::checkbox(self.config.chord_mode == board::ChordMode::LeftClick)
                        .label("Left-click chord")
                        .style(move |_, _| check_box_style)
                        .on_toggle(MainWindowMessage::ChordModeToggled)
                )
            ]
            .spacing(4)
            .padding(4),
        )
        .width(iced::Length::Fixed(150.0))
        .style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(base_color)),
            ..Default::default()
        });
        iced::widget::scrollable(iced::widget::row![
            control_panel,
            if let Some(game) = &self.game {
                game.view().map(game::GameMessage::into)
            } else {
                iced::widget::Text::new("Failed to initialize the game, please check logs for details.")
                    .color(iced::Color::from_rgb8(255, 0, 0))
                    .into()
            }
        ])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .direction(iced::widget::scrollable::Direction::Both {
            vertical: Default::default(),
            horizontal: Default::default(),
        })
        .on_scroll(MainWindowMessage::Scrolled)
        .into()
    }

    pub fn subscriptions(&self) -> iced::Subscription<MainWindowMessage> {
        iced::event::listen_with(|event, _, _| match event {
            iced::Event::Window(iced::window::Event::Opened { size, .. }) => Some(MainWindowMessage::Game(
                game::GameMessage::ViewportChanged(iced::Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: size.width,
                    height: size.height,
                }),
            )),
            _ => None,
        })
    }
}
