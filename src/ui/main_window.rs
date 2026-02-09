use std::sync::Arc;

use iced::Task;
use log::{debug, error};

use crate::{
    config::GlobalConfig,
    ui::{board_area::BoardArea, board_frame::BoardFrame, *},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BaseWindow {
    #[default]
    Player,
}

pub struct MainWindow {
    skin_manager: skin::SkinManager,
    skin: Arc<skin::Skin>,
    theme: iced::Theme,
    board_frame: BoardFrame,
    player: player::Player,
    base_window: BaseWindow,
}

impl MainWindow {
    pub fn new(config: GlobalConfig) -> crate::error::Result<Self> {
        let skin_manager = crate::utils::resource_path("skin")
            .inspect_err(|e| error!("Failed to get skin resource path: {}", e))
            .and_then(|path| {
                skin::SkinManager::new(path).inspect_err(|e| error!("Failed to initialize SkinManager: {}", e))
            })?;
        let skin = Arc::new(
            skin_manager
                .skins()
                .get(&config.skin)
                .ok_or_else(|| {
                    error!("Skin '{}' not found.", config.skin);
                    crate::error::Error::SkinNotFound(config.skin.clone())
                })?
                .build(config.cell_size)
                .inspect_err(|e| error!("Failed to build skin '{}': {}", config.skin, e))?,
        );
        let theme = if skin.light {
            iced::Theme::Light
        } else {
            iced::Theme::Dark
        };

        let board_area = BoardArea::calculate(&skin, config.cell_size, config.board[0], config.board[1]);
        let board_frame = BoardFrame::new(board_area, Arc::clone(&skin));
        let player = player::Player::new(config.clone(), board_area, Arc::clone(&skin));

        Ok(Self {
            skin_manager,
            skin,
            theme,
            board_frame,
            player,
            base_window: BaseWindow::default(),
        })
    }

    pub fn update(&mut self, msg: AppMessage) -> Task<AppMessage> {
        match msg {
            AppMessage::Player(player::PlayerMessage::Request(request)) => match request {
                player::RequestMessage::RegenerateSkin { skin, cell_size } => {
                    debug!("Regenerating skin: {}, cell size: {}", skin, cell_size);
                    let skin_builder = self.skin_manager.skins().get(&skin).or_else(|| {
                        error!("Skin '{}' not found.", skin);
                        None
                    });
                    if let Some(skin_builder) = skin_builder {
                        let Ok(skin) = skin_builder
                            .build(cell_size)
                            .inspect_err(|e| error!("Failed to build skin '{}': {}", skin, e))
                        else {
                            return Task::none();
                        };
                        self.skin = Arc::new(skin);
                        return self
                            .player
                            .update(player::PlayerMessage::UpdateSkin(Arc::clone(&self.skin)))
                            .map(AppMessage::Player);
                    };
                },
                player::RequestMessage::UpdateBoardArea(board_area) => {
                    debug!("Updating board area: {:?}", board_area);
                    self.board_frame = BoardFrame::new(board_area, Arc::clone(&self.skin));
                },
                _ => {},
            },
            AppMessage::Player(player_msg) => {
                return self.player.update(player_msg).map(AppMessage::Player);
            },
            _ => {},
        }
        Task::none()
    }

    pub fn view(&self) -> iced::Element<'_, AppMessage> {
        match self.base_window {
            BaseWindow::Player => iced::widget::scrollable(iced::widget::row![
                self.player.view_sidebar(200.0).map(AppMessage::Player),
                iced::widget::Stack::with_capacity(2)
                    .push(self.board_frame.view())
                    .push(self.player.view_game().map(AppMessage::Player)),
            ])
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .direction(iced::widget::scrollable::Direction::Both {
                vertical: Default::default(),
                horizontal: Default::default(),
            })
            .on_scroll(|viewport| AppMessage::Player(player::PlayerMessage::Scrolled(viewport))),
        }
        .into()
    }

    pub fn theme(&self) -> Option<iced::Theme> {
        Some(self.theme.clone())
    }

    pub fn subscriptions(&self) -> iced::Subscription<AppMessage> {
        self.player.subscriptions().map(AppMessage::Player)
    }
}
