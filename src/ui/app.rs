use std::sync::Arc;

use iced::Task;
use log::{debug, error, info, trace, warn};

use crate::{
    base::*,
    config::*,
    ui::{
        player::{Player, PlayerMessage},
        *,
    },
};

use iced::futures::StreamExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BaseWindow {
    #[default]
    Player,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    GetWindowId(Option<iced::window::Id>),
    Modal(modal::ModalMessage),
    Player(PlayerMessage),
    CloseWindow(iced::window::Id),
    ActivateWindow,
}

pub struct App {
    config: GlobalConfig,
    id: Option<iced::window::Id>,
    skin_manager: Option<skin::SkinManager>,
    skin: Option<Arc<skin::Skin>>,
    theme: iced::Theme,
    base_window: BaseWindow,
    current_modal: modal::Modal,
    player: Option<player::Player>,
    export: modal::export::ExportModal,
    import: modal::import::ImportModal,
    error: modal::error::ErrorModal,
}

impl App {
    pub fn new() -> (Self, Task<AppMessage>) {
        let config = GlobalConfig::load().unwrap_or_else(|err| {
            warn!("Failed to load config, using default config.");
            GlobalConfig {
                chord_mode: board::ChordMode::LeftClick,
                skin: "WoM Light".to_string(),
                cell_size: 24,
                board: [30, 16, 99],
            }
        });
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
                    .map(Arc::new)
            });
        let theme = if skin.as_ref().map(|s| s.light).unwrap_or_default() {
            iced::Theme::Light
        } else {
            iced::Theme::Dark
        };
        let player = skin.as_ref().map(|skin| Player::new(config.clone(), Arc::clone(skin)));

        let mut error = modal::error::ErrorModal::new();
        let mut current_modal = modal::Modal::default();
        if player.is_none() {
            trace!("Player failed to initialize, showing error modal.");
            current_modal = modal::Modal::Error;
            error.error_message = "Failed to initialize. Please check the logs for details.".to_string();
        }
        (
            Self {
                config,
                id: None,
                skin_manager,
                skin,
                theme,
                base_window: BaseWindow::default(),
                current_modal,
                player,
                export: modal::export::ExportModal::new(),
                import: modal::import::ImportModal::new(),
                error,
            },
            iced::window::latest().map(AppMessage::GetWindowId),
        )
    }

    fn modal<'a, Message>(
        base: impl Into<iced::Element<'a, Message>>,
        content: impl Into<iced::Element<'a, Message>>,
        on_blur: Message,
    ) -> iced::Element<'a, Message>
    where
        Message: Clone + 'a,
    {
        iced::widget::stack![
            base.into(),
            iced::widget::opaque(
                iced::widget::mouse_area(iced::widget::center(iced::widget::opaque(content)).style(|_theme| {
                    iced::widget::container::Style {
                        background: Some(
                            iced::Color {
                                a: 0.8,
                                ..iced::Color::BLACK
                            }
                            .into(),
                        ),
                        ..iced::widget::container::Style::default()
                    }
                }))
                .on_press(on_blur)
            )
        ]
        .into()
    }

    pub fn update(&mut self, msg: AppMessage) -> Task<AppMessage> {
        trace!("AppMessage received: {:?}", msg);
        match msg {
            AppMessage::GetWindowId(id) => {
                debug!("Window ID obtained: {:?}", id);
                self.id = id;
            },
            AppMessage::ActivateWindow => {
                info!("Activating window due to second instance launch");
                return match self.id {
                    Some(id) => iced::window::gain_focus(id),
                    // Get the oldest (main) window and focus it
                    None => iced::window::oldest().and_then(iced::window::gain_focus),
                };
            },
            AppMessage::Player(PlayerMessage::Request(request)) => {
                trace!("Handling player request: {:?}", request);
                let Some(player) = &mut self.player else {
                    error!("Player instance is not initialized.");
                    return Task::none();
                };
                match request {
                    player::RequestMessage::SyncConfigToApp(config) => {
                        debug!("Applying config update: {:?}", config);
                        config.apply_to(&mut self.config);
                    },
                    player::RequestMessage::RegenerateSkin { skin, cell_size } => {
                        self.config.skin = skin;
                        self.config.cell_size = cell_size;
                        self.skin = self
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
                                    .map(Arc::new)
                            });
                        if let Some(skin) = &self.skin {
                            return player
                                .update(player::PlayerMessage::UpdateSkin(Arc::clone(skin)))
                                .map(AppMessage::Player);
                        }
                    },
                    player::RequestMessage::ShowImportModal => {
                        debug!("Showing import modal");
                        self.current_modal = modal::Modal::ImportGame;
                    },
                    player::RequestMessage::ShowExportModal => {
                        debug!("Showing export modal");
                        self.current_modal = modal::Modal::ExportGame;
                    },
                    player::RequestMessage::ShowErrorModal(err) => {
                        debug!("Showing error modal: {}", err);
                        self.error.error_message = err;
                        self.current_modal = modal::Modal::Error;
                    },
                }
            },
            AppMessage::Player(msg) => {
                let Some(player) = &mut self.player else {
                    error!("Player instance is not initialized.");
                    return Task::none();
                };
                return player.update(msg).map(AppMessage::Player);
            },
            AppMessage::Modal(modal::ModalMessage::Import(modal::import::ImportMessage::Cancel))
            | AppMessage::Modal(modal::ModalMessage::Export(modal::export::ExportMessage::Cancel))
            | AppMessage::Modal(modal::ModalMessage::Error(modal::error::ErrorMessage::Acknowledge)) => {
                debug!("Closing modal: {:?}", self.current_modal);
                self.current_modal = modal::Modal::None;
            },
            AppMessage::Modal(modal::ModalMessage::Import(msg)) => {
                trace!("Handling import modal message: {:?}", msg);
                if let modal::import::ImportMessage::Confirm = msg {
                    debug!("Import modal confirmed");
                    self.current_modal = modal::Modal::None;
                    let import_type = self.import.config.import_type;
                    let text = std::mem::take(&mut self.import.config.text);
                    self.import.update(msg);
                    if let Some(player) = &mut self.player {
                        return player
                            .update(PlayerMessage::Import(player::ImportMessage::StartImport(
                                import_type,
                                text.text(),
                            )))
                            .map(AppMessage::Player);
                    }
                } else {
                    self.import.update(msg);
                }
            },
            AppMessage::Modal(modal::ModalMessage::Export(msg)) => {
                trace!("Handling export modal message: {:?}", msg);
                if let modal::export::ExportMessage::Confirm = msg {
                    debug!("Export modal confirmed");
                    self.current_modal = modal::Modal::None;
                    let export_type = self.export.config.export_type;
                    self.export.update(msg);
                    if let Some(player) = &mut self.player {
                        return player
                            .update(PlayerMessage::Export(player::ExportMessage::StartExport(export_type)))
                            .map(AppMessage::Player);
                    }
                } else {
                    self.export.update(msg);
                }
            },
            AppMessage::CloseWindow(id) => match self.id {
                Some(main_id) if main_id != id => {
                    debug!("Ignoring close request for non-main window: {:?}", id);
                },
                _ => {
                    debug!("Saving config on exit: {:?}", self.config);
                    _ = self.config.save();
                    return iced::exit();
                },
            },
        }
        Task::none()
    }

    pub fn view(&self) -> iced::Element<'_, AppMessage> {
        let base = match self.base_window {
            BaseWindow::Player => self.player.as_ref().map(|player| player.view().map(AppMessage::Player)),
        }
        .unwrap_or(iced::widget::space().height(iced::Fill).width(iced::Fill).into());

        trace!("Rendering view with current modal: {:?}", self.current_modal);
        match self.current_modal {
            modal::Modal::ImportGame => App::modal(
                base,
                self.import
                    .view()
                    .map(modal::ModalMessage::Import)
                    .map(AppMessage::Modal),
                AppMessage::Modal(modal::ModalMessage::Import(modal::import::ImportMessage::Cancel)),
            ),
            modal::Modal::ExportGame => App::modal(
                base,
                self.export
                    .view()
                    .map(modal::ModalMessage::Export)
                    .map(AppMessage::Modal),
                AppMessage::Modal(modal::ModalMessage::Export(modal::export::ExportMessage::Cancel)),
            ),
            modal::Modal::Error => App::modal(
                base,
                self.error.view().map(modal::ModalMessage::Error).map(AppMessage::Modal),
                AppMessage::Modal(modal::ModalMessage::Error(modal::error::ErrorMessage::Acknowledge)),
            ),
            modal::Modal::None => base,
        }
    }

    pub fn theme(&self) -> Option<iced::Theme> {
        Some(self.theme.clone())
    }

    pub fn subscriptions(&self) -> iced::Subscription<AppMessage> {
        let mut subscriptions = Vec::with_capacity(3);
        subscriptions.push(iced::window::close_requests().map(AppMessage::CloseWindow));
        if let Some(player) = &self.player {
            subscriptions.push(player.subscriptions().map(AppMessage::Player));
        }
        subscriptions.push(crate::single_instance::activation_subscription());
        iced::Subscription::batch(subscriptions)
    }
}
