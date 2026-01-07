pub mod game;
pub mod overlay;
pub mod skin;

use crate::{base::board, config::GlobalConfig, engine::analysis, ui::overlay::AnalysisOverlayMessage};
use iced::{Function, Task};
use log::{debug, error, trace};
use std::ops::Not;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputType {
    Width = 0,
    Height = 1,
    Mines = 2,
    CellSize = 3,
}

#[derive(Debug, Clone)]
pub enum MainWindowMessage {
    Game(game::GameMessage),
    TextInputChanged(TextInputType, String),
    CellSizeSubmit,
    ChordModeToggled(bool),
    Scrolled(iced::widget::scrollable::Viewport),
    Analysis(overlay::AnalysisOverlayMessage),
}

impl From<game::GameMessage> for MainWindowMessage {
    fn from(message: game::GameMessage) -> Self {
        MainWindowMessage::Game(message)
    }
}

impl From<overlay::AnalysisOverlayMessage> for MainWindowMessage {
    fn from(message: overlay::AnalysisOverlayMessage) -> Self {
        MainWindowMessage::Analysis(message)
    }
}

pub struct MainWindow {
    config: GlobalConfig,
    skin_manager: Option<skin::SkinManager>,
    skin: Option<skin::Skin>,
    game: Option<game::Game>,
    text_input_states: [String; 4],
    analysis_overlay: overlay::AnalysisOverlay,
    viewport: iced::Rectangle,
    update_analysis_in_progress: bool,
    update_analysis_scheduled: bool,
}

impl MainWindow {
    const TEXT_INPUT_UPPER: [usize; 4] = [1000, 1000, 1000000, 64];
    const TEXT_INPUT_LOWER: [usize; 4] = [1, 1, 1, 8];
    const TEXT_INPUT_DEFAULTS: [usize; 4] = [30, 16, 99, 24];

    pub fn new() -> (Self, Task<MainWindowMessage>) {
        let config = GlobalConfig {
            chord_mode: board::ChordMode::LeftClick,
            skin: "WoM Light".to_string(),
            cell_size: 24,
            board: [30, 16, 99],
            show_probabilities: false,
            analysis_admit_flags: true,
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
        let game = skin.clone().map(|skin| game::Game::new(board, &config, skin));
        let text_input_states = [
            config.board[0].to_string(),
            config.board[1].to_string(),
            config.board[2].to_string(),
            config.cell_size.to_string(),
        ];
        let analysis_overlay = overlay::AnalysisOverlay::new(
            crate::engine::analysis::default_engine(),
            config.analysis_admit_flags,
            game.as_ref().map(|game| game.game_area()).unwrap_or_default(),
            game.as_ref().map(|game| game.board_area()).unwrap_or_default(),
            iced::Rectangle::default(),
            config.cell_size,
            config.show_probabilities,
            skin.as_ref().is_some_and(|s| s.light),
        );
        (
            Self {
                config,
                skin_manager,
                skin,
                game,
                text_input_states,
                viewport: Default::default(),
                analysis_overlay,
                update_analysis_in_progress: false,
                update_analysis_scheduled: false,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: MainWindowMessage) -> iced::Task<MainWindowMessage> {
        match message {
            MainWindowMessage::Game(msg) => {
                let is_face_clicked = matches!(msg, game::GameMessage::FaceClicked);

                if let game::GameMessage::ViewportChanged(viewport) = msg {
                    self.viewport = viewport;
                    self.analysis_overlay.set_viewport(viewport);
                }

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
                            .map(|skin| game::Game::new(new_board, &self.config, skin))
                            .inspect(|game| {
                                self.config.board = [game.board().width(), game.board().height(), game.board().mines()];
                                self.text_input_states = [
                                    self.config.board[0].to_string(),
                                    self.config.board[1].to_string(),
                                    self.config.board[2].to_string(),
                                    self.config.cell_size.to_string(),
                                ];
                                self.analysis_overlay.clear_analysis();
                                self.analysis_overlay.update(overlay::AnalysisOverlayMessage::Resize {
                                    cell_size: self.config.cell_size,
                                    game_area: game.game_area(),
                                    board_area: game.board_area(),
                                });
                            });
                        if let Some(game) = &mut self.game {
                            game.update(game::GameMessage::ViewportChanged(self.viewport));
                        }
                        return Task::none();
                    }
                }

                if let Some(game) = &mut self.game {
                    let should_update_analysis = game.update(msg);
                    if is_face_clicked {
                        self.config.board = [game.board().width(), game.board().height(), game.board().mines()];
                        self.text_input_states = [
                            self.config.board[0].to_string(),
                            self.config.board[1].to_string(),
                            self.config.board[2].to_string(),
                            self.config.cell_size.to_string(),
                        ];
                    }
                    if should_update_analysis && let Some(task) = self.update_analysis() {
                        return task;
                    }
                }
            },
            MainWindowMessage::TextInputChanged(input_type, value) => {
                if value.is_empty() {
                    self.text_input_states[input_type as usize] = value;
                    if input_type == TextInputType::CellSize {
                        self.config.cell_size = if let Some(game) = &self.game {
                            game.cell_size()
                        } else {
                            MainWindow::TEXT_INPUT_DEFAULTS[TextInputType::CellSize as usize] as u32
                        };
                    } else {
                        self.config.board[input_type as usize] = if let Some(game) = &self.game {
                            match input_type {
                                TextInputType::Width => game.board().width(),
                                TextInputType::Height => game.board().height(),
                                TextInputType::Mines => game.board().mines(),
                                _ => unreachable!(),
                            }
                        } else {
                            MainWindow::TEXT_INPUT_DEFAULTS[input_type as usize]
                        };
                    }
                    return Task::none();
                }
                if let Ok(mut num) = value.parse::<usize>() {
                    num = num.clamp(1, MainWindow::TEXT_INPUT_UPPER[input_type as usize]);
                    self.text_input_states[input_type as usize] = num.to_string();
                    if input_type == TextInputType::CellSize {
                        self.config.cell_size = num as u32;
                    } else {
                        self.config.board[input_type as usize] = num;
                    }
                    trace!(
                        "Text input changed: {:?} = {}, updated board config to {:?}",
                        input_type, num, self.config.board
                    );
                }
            },
            MainWindowMessage::CellSizeSubmit => {
                self.config.cell_size = self
                    .config
                    .cell_size
                    .max(MainWindow::TEXT_INPUT_LOWER[TextInputType::CellSize as usize] as u32);
                self.text_input_states[TextInputType::CellSize as usize] = self.config.cell_size.to_string();
                trace!("Cell size submitted: {}", self.config.cell_size);
                let skin = self
                    .skin_manager
                    .as_ref()
                    .and_then(|manager| {
                        manager.skins().get(&self.config.skin).or_else(|| {
                            error!("Skin '{}' not found.", self.config.skin);
                            None
                        })
                    })
                    .and_then(|builder| {
                        builder
                            .build(self.config.cell_size)
                            .inspect_err(|e| error!("Failed to build skin '{}': {}", self.config.skin, e))
                            .ok()
                    });
                if let Some(skin) = skin {
                    self.skin = Some(skin.clone());
                }

                if let Some(game) = &mut self.game
                    && let Some(skin) = &self.skin
                {
                    game.update(game::GameMessage::Resize(self.config.cell_size, Box::new(skin.clone())));
                    self.analysis_overlay.update(overlay::AnalysisOverlayMessage::Resize {
                        cell_size: self.config.cell_size,
                        game_area: game.game_area(),
                        board_area: game.board_area(),
                    });
                }
            },
            MainWindowMessage::ChordModeToggled(enabled) => {
                self.config.chord_mode = if enabled {
                    board::ChordMode::LeftClick
                } else {
                    board::ChordMode::Standard
                };
                trace!("Chord mode toggled: {:?}", self.config.chord_mode);
                if let Some(game) = &mut self.game {
                    game.update(game::GameMessage::ChordModeChanged(self.config.chord_mode));
                }
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
                    self.viewport = viewport_rect;
                    game.update(game::GameMessage::ViewportChanged(viewport_rect));
                    self.analysis_overlay.set_viewport(self.viewport);
                }
            },
            MainWindowMessage::Analysis(msg) => match &msg {
                AnalysisOverlayMessage::SetEnabled(true) => {
                    self.config.show_probabilities = true;
                    self.analysis_overlay.update(msg);
                    if let Some(task) = self.update_analysis() {
                        return task;
                    }
                },
                AnalysisOverlayMessage::SetEnabled(false) => {
                    self.config.show_probabilities = false;
                    self.update_analysis_in_progress = false;
                    self.update_analysis_scheduled = false;
                    self.analysis_overlay.update(msg);
                },
                AnalysisOverlayMessage::SetAdmitFlags(admit_flags) => {
                    self.config.analysis_admit_flags = *admit_flags;
                    self.analysis_overlay.update(msg);
                    if let Some(task) = self.update_analysis() {
                        return task;
                    }
                },
                AnalysisOverlayMessage::AnalysisCompleted(..) => {
                    self.analysis_overlay.update(msg);
                    self.update_analysis_in_progress = false;
                    if self.update_analysis_scheduled {
                        trace!("Running scheduled analysis update");
                        self.update_analysis_scheduled = false;
                        if let Some(task) = self.update_analysis() {
                            return task;
                        }
                    }
                },
                _ => self.analysis_overlay.update(msg),
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
        let button_style_press = iced::widget::button::Style {
            background: Some(iced::Background::Color(base_color)),
            border: iced::Border {
                color: upper_color,
                width: 2.0,
                radius: iced::border::radius(4.0),
            },
            text_color,
            ..Default::default()
        };
        let button_style_disabled = iced::widget::button::Style {
            background: Some(iced::Background::Color(base_color)),
            border: iced::Border {
                color: upper_color,
                width: 2.0,
                radius: iced::border::radius(4.0),
            },
            text_color: upper_color,
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
        let check_box_style_disabled = iced::widget::checkbox::Style {
            background: iced::Background::Color(iced::Color::TRANSPARENT),
            border: iced::Border {
                color: upper_color,
                width: 2.0,
                radius: iced::border::radius(4.0),
            },
            icon_color: upper_color,
            text_color: Some(upper_color),
        };

        let board_control = iced::widget::container(
            iced::widget::column![
                iced::widget::center_x(iced::widget::text("Board Config").size(18).color(text_color)),
                iced::widget::row![
                    iced::widget::text("Width:")
                        .color(text_color)
                        .size(16)
                        .width(iced::FillPortion(1)),
                    iced::widget::TextInput::new("width", &self.text_input_states[TextInputType::Width as usize])
                        .width(iced::FillPortion(2))
                        .on_input(MainWindowMessage::TextInputChanged.with(TextInputType::Width))
                        .style(move |_, _| text_input_style)
                ]
                .align_y(iced::alignment::Vertical::Center),
                iced::widget::row![
                    iced::widget::text("Height:")
                        .color(text_color)
                        .size(16)
                        .width(iced::FillPortion(1)),
                    iced::widget::TextInput::new("height", &self.text_input_states[TextInputType::Height as usize])
                        .width(iced::FillPortion(2))
                        .on_input(MainWindowMessage::TextInputChanged.with(TextInputType::Height))
                        .style(move |_, _| text_input_style)
                ]
                .align_y(iced::alignment::Vertical::Center),
                iced::widget::row![
                    iced::widget::text("Mines:")
                        .color(text_color)
                        .size(16)
                        .width(iced::FillPortion(1)),
                    iced::widget::TextInput::new("mines", &self.text_input_states[TextInputType::Mines as usize])
                        .width(iced::FillPortion(2))
                        .on_input(MainWindowMessage::TextInputChanged.with(TextInputType::Mines))
                        .style(move |_, _| text_input_style)
                ]
                .align_y(iced::alignment::Vertical::Center),
                iced::widget::center_x(
                    iced::widget::button(iced::widget::text("New Game").align_x(iced::alignment::Horizontal::Center))
                        .width(120.0)
                        .style(move |_, state| if state == iced::widget::button::Status::Pressed {
                            button_style_press
                        } else {
                            button_style
                        })
                        .on_press(MainWindowMessage::Game(game::GameMessage::FaceClicked))
                ),
                iced::widget::center_x(
                    iced::widget::button(iced::widget::text("Continue").align_x(iced::alignment::Horizontal::Center))
                        .width(120.0)
                        .style(move |_, state| {
                            if state == iced::widget::button::Status::Disabled {
                                button_style_disabled
                            } else if state == iced::widget::button::Status::Pressed {
                                button_style_press
                            } else {
                                button_style
                            }
                        })
                        .on_press_maybe(self.game.as_ref().and_then(|game| {
                            matches!(game.board().state(), board::BoardState::Lost { .. })
                                .then_some(MainWindowMessage::Game(game::GameMessage::Continue))
                        }))
                ),
                iced::widget::center_x(
                    iced::widget::button(iced::widget::text("Replay").align_x(iced::alignment::Horizontal::Center))
                        .width(120.0)
                        .style(move |_, state| {
                            if state == iced::widget::button::Status::Disabled {
                                button_style_disabled
                            } else if state == iced::widget::button::Status::Pressed {
                                button_style_press
                            } else {
                                button_style
                            }
                        })
                        .on_press_maybe(self.game.as_ref().and_then(|game| {
                            matches!(game.board().state(), board::BoardState::NotStarted)
                                .not()
                                .then_some(MainWindowMessage::Game(game::GameMessage::Replay))
                        }))
                ),
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
        let cell_size = iced::widget::row![
            iced::widget::text("Cell Size:")
                .color(text_color)
                .size(16)
                .width(iced::FillPortion(1)),
            iced::widget::TextInput::new("cell size", &self.text_input_states[TextInputType::CellSize as usize])
                .width(iced::FillPortion(1))
                .on_input(MainWindowMessage::TextInputChanged.with(TextInputType::CellSize))
                .style(move |_, _| text_input_style)
                .on_submit(MainWindowMessage::CellSizeSubmit),
            iced::widget::button("↵")
                .style(move |_, state| if state == iced::widget::button::Status::Pressed {
                    button_style_press
                } else {
                    button_style
                })
                .on_press(MainWindowMessage::CellSizeSubmit)
                .width(iced::Length::Shrink)
        ]
        .align_y(iced::alignment::Vertical::Center);
        let control_panel = iced::widget::container(
            iced::widget::column![
                iced::widget::center_x(iced::widget::text("Control Panel").size(20).color(text_color)),
                iced::widget::center_x(board_control),
                iced::widget::checkbox(self.config.chord_mode == board::ChordMode::LeftClick)
                    .label("Left-click chord")
                    .style(move |_, _| check_box_style)
                    .on_toggle(MainWindowMessage::ChordModeToggled),
                iced::widget::checkbox(self.config.show_probabilities)
                    .label("Show Probability")
                    .style(move |_, _| check_box_style)
                    .on_toggle(
                        |enabled| MainWindowMessage::Analysis(overlay::AnalysisOverlayMessage::SetEnabled(enabled))
                    ),
                iced::widget::checkbox(self.config.analysis_admit_flags)
                    .label("Admits Flags")
                    .style(
                        move |_, state| if matches!(state, iced::widget::checkbox::Status::Disabled { .. }) {
                            check_box_style_disabled
                        } else {
                            check_box_style
                        }
                    )
                    .on_toggle_maybe(self.config.show_probabilities.then_some(|admit_flags| {
                        MainWindowMessage::Analysis(overlay::AnalysisOverlayMessage::SetAdmitFlags(admit_flags))
                    })),
                cell_size
            ]
            .spacing(4)
            .padding(4),
        )
        .width(iced::Length::Fixed(200.0))
        .style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(base_color)),
            ..Default::default()
        });
        iced::widget::scrollable(if let Some(game) = &self.game {
            iced::widget::row![
                control_panel,
                iced::widget::Stack::with_capacity(2)
                    .push(game.view().map(game::GameMessage::into))
                    .push(self.analysis_overlay.view())
            ]
        } else {
            iced::widget::row![
                control_panel,
                iced::widget::text("Failed to initialize the game, please check logs for details.")
                    .color(iced::Color::from_rgb8(255, 0, 0))
            ]
        })
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
            iced::Event::Window(iced::window::Event::Opened { size, .. }) => {
                trace!("Window opened with size: {:?}", size);
                Some(MainWindowMessage::Game(game::GameMessage::ViewportChanged(
                    iced::Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width: size.width,
                        height: size.height,
                    },
                )))
            },
            iced::Event::Window(iced::window::Event::Resized(size)) => {
                trace!("Window resized to size: {:?}", size);
                Some(MainWindowMessage::Game(game::GameMessage::ViewportChanged(
                    iced::Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width: size.width,
                        height: size.height,
                    },
                )))
            },
            _ => None,
        })
    }

    fn update_analysis(&mut self) -> Option<iced::Task<MainWindowMessage>> {
        if self.config.show_probabilities
            && let Some(game) = &self.game
        {
            if self.update_analysis_in_progress {
                trace!("Analysis update already in progress, scheduling another update");
                self.update_analysis_scheduled = true;
                return None;
            }
            trace!("Starting analysis update");
            self.update_analysis_in_progress = true;
            return Some(
                self.analysis_overlay
                    .update_analysis(game.board())
                    .map(MainWindowMessage::Analysis),
            );
        }
        None
    }
}
