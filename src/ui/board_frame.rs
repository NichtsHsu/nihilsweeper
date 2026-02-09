use std::sync::Arc;

use crate::ui::{AppMessage, board_area::BoardArea, skin};
use iced::widget::canvas;

pub struct BoardFrame {
    board_area: BoardArea,
    skin: Arc<skin::Skin>,
    borders: Vec<iced::Rectangle>,
    light_paths: Vec<canvas::Path>,
    shadow_paths: Vec<canvas::Path>,
    cache: canvas::Cache,
}

impl BoardFrame {
    pub fn new(board_area: BoardArea, skin: Arc<skin::Skin>) -> Self {
        let mut borders = vec![
            iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: skin.border.width,
                height: board_area.canvas_area.height,
            },
            iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: board_area.canvas_area.width,
                height: skin.border.width,
            },
            iced::Rectangle {
                x: board_area.canvas_area.x + board_area.canvas_area.width - skin.border.width,
                y: 0.0,
                width: skin.border.width,
                height: board_area.canvas_area.height,
            },
            iced::Rectangle {
                x: 0.0,
                y: board_area.canvas_area.y + board_area.canvas_area.height - skin.border.width,
                width: board_area.canvas_area.width,
                height: skin.border.width,
            },
        ];

        let light_shadow_width = skin.border.width / 4.0;
        let main_light_path = canvas::Path::new(|p| {
            p.move_to(board_area.canvas_area.position());
            p.line_to(iced::Point::new(
                board_area.canvas_area.x + board_area.canvas_area.width,
                board_area.canvas_area.y,
            ));
            p.line_to(iced::Point::new(
                board_area.canvas_area.x + board_area.canvas_area.width - light_shadow_width,
                board_area.canvas_area.y + light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                board_area.canvas_area.x + light_shadow_width,
                board_area.canvas_area.y + light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                board_area.canvas_area.x + light_shadow_width,
                board_area.canvas_area.y + board_area.canvas_area.height - light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                board_area.canvas_area.x,
                board_area.canvas_area.y + board_area.canvas_area.height,
            ));
            p.close();
        });
        let main_shadow_path = canvas::Path::new(|p| {
            p.move_to(iced::Point::new(
                board_area.canvas_area.x + board_area.canvas_area.width,
                board_area.canvas_area.y,
            ));
            p.line_to(iced::Point::new(
                board_area.canvas_area.x + board_area.canvas_area.width,
                board_area.canvas_area.y + board_area.canvas_area.height,
            ));
            p.line_to(iced::Point::new(
                board_area.canvas_area.x,
                board_area.canvas_area.y + board_area.canvas_area.height,
            ));
            p.line_to(iced::Point::new(
                board_area.canvas_area.x + light_shadow_width,
                board_area.canvas_area.y + board_area.canvas_area.height - light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                board_area.canvas_area.x + board_area.canvas_area.width - light_shadow_width,
                board_area.canvas_area.y + board_area.canvas_area.height - light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                board_area.canvas_area.x + board_area.canvas_area.width - light_shadow_width,
                board_area.canvas_area.y + light_shadow_width,
            ));
            p.close();
        });
        let board_light_path = canvas::Path::new(|p| {
            p.move_to(iced::Point::new(
                board_area.game_area.x + board_area.game_area.width,
                board_area.game_area.y,
            ));
            p.line_to(iced::Point::new(
                board_area.game_area.x + board_area.game_area.width,
                board_area.game_area.y + board_area.game_area.height,
            ));
            p.line_to(iced::Point::new(
                board_area.game_area.x,
                board_area.game_area.y + board_area.game_area.height,
            ));
            p.line_to(iced::Point::new(
                board_area.game_area.x - light_shadow_width,
                board_area.game_area.y + board_area.game_area.height + light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                board_area.game_area.x + board_area.game_area.width + light_shadow_width,
                board_area.game_area.y + board_area.game_area.height + light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                board_area.game_area.x + board_area.game_area.width + light_shadow_width,
                board_area.game_area.y - light_shadow_width,
            ));
            p.close();
        });
        let board_shadow_path = canvas::Path::new(|p| {
            p.move_to(board_area.game_area.position());
            p.line_to(iced::Point::new(
                board_area.game_area.x + board_area.game_area.width,
                board_area.game_area.y,
            ));
            p.line_to(iced::Point::new(
                board_area.game_area.x + board_area.game_area.width + light_shadow_width,
                board_area.game_area.y - light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                board_area.game_area.x - light_shadow_width,
                board_area.game_area.y - light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                board_area.game_area.x - light_shadow_width,
                board_area.game_area.y + board_area.game_area.height + light_shadow_width,
            ));
            p.line_to(iced::Point::new(
                board_area.game_area.x,
                board_area.game_area.y + board_area.game_area.height,
            ));
            p.close();
        });

        let mut light_paths = vec![main_light_path, board_light_path];
        let mut shadow_paths = vec![main_shadow_path, board_shadow_path];

        if board_area.top_area != iced::Rectangle::default() {
            borders.push(iced::Rectangle {
                x: 0.0,
                y: board_area.top_area.y + board_area.top_area.height,
                width: board_area.canvas_area.width,
                height: skin.border.width,
            });
            let top_light_path = canvas::Path::new(|p| {
                p.move_to(iced::Point::new(
                    board_area.top_area.x + board_area.top_area.width,
                    board_area.top_area.y,
                ));
                p.line_to(iced::Point::new(
                    board_area.top_area.x + board_area.top_area.width,
                    board_area.top_area.y + board_area.top_area.height,
                ));
                p.line_to(iced::Point::new(
                    board_area.top_area.x,
                    board_area.top_area.y + board_area.top_area.height,
                ));
                p.line_to(iced::Point::new(
                    board_area.top_area.x - light_shadow_width,
                    board_area.top_area.y + board_area.top_area.height + light_shadow_width,
                ));
                p.line_to(iced::Point::new(
                    board_area.top_area.x + board_area.top_area.width + light_shadow_width,
                    board_area.top_area.y + board_area.top_area.height + light_shadow_width,
                ));
                p.line_to(iced::Point::new(
                    board_area.top_area.x + board_area.top_area.width + light_shadow_width,
                    board_area.top_area.y - light_shadow_width,
                ));
                p.close();
            });
            let top_shadow_path = canvas::Path::new(|p| {
                p.move_to(board_area.top_area.position());
                p.line_to(iced::Point::new(
                    board_area.top_area.x + board_area.top_area.width,
                    board_area.top_area.y,
                ));
                p.line_to(iced::Point::new(
                    board_area.top_area.x + board_area.top_area.width + light_shadow_width,
                    board_area.top_area.y - light_shadow_width,
                ));
                p.line_to(iced::Point::new(
                    board_area.top_area.x - light_shadow_width,
                    board_area.top_area.y - light_shadow_width,
                ));
                p.line_to(iced::Point::new(
                    board_area.top_area.x - light_shadow_width,
                    board_area.top_area.y + board_area.top_area.height + light_shadow_width,
                ));
                p.line_to(iced::Point::new(
                    board_area.top_area.x,
                    board_area.top_area.y + board_area.top_area.height,
                ));
                p.close();
            });
            light_paths.push(top_light_path);
            shadow_paths.push(top_shadow_path);
        }

        let counter_border_width = ((skin.top_area.counter.height
            - skin.top_area.counter.content_height
            - skin.top_area.counter.content_gap * 2.0)
            / 2.0)
            .floor();
        if board_area.counter_area != iced::Rectangle::default() && counter_border_width > 0.0 {
            let counter_light_path = canvas::Path::new(|p| {
                p.move_to(iced::Point::new(
                    board_area.counter_area.x + board_area.counter_area.width,
                    board_area.counter_area.y,
                ));
                p.line_to(iced::Point::new(
                    board_area.counter_area.x + board_area.counter_area.width,
                    board_area.counter_area.y + board_area.counter_area.height,
                ));
                p.line_to(iced::Point::new(
                    board_area.counter_area.x,
                    board_area.counter_area.y + board_area.counter_area.height,
                ));
                p.line_to(iced::Point::new(
                    board_area.counter_area.x - counter_border_width,
                    board_area.counter_area.y + board_area.counter_area.height + counter_border_width,
                ));
                p.line_to(iced::Point::new(
                    board_area.counter_area.x + board_area.counter_area.width + counter_border_width,
                    board_area.counter_area.y + board_area.counter_area.height + counter_border_width,
                ));
                p.line_to(iced::Point::new(
                    board_area.counter_area.x + board_area.counter_area.width + counter_border_width,
                    board_area.counter_area.y - counter_border_width,
                ));
                p.close();
            });
            let counter_shadow_path = canvas::Path::new(|p| {
                p.move_to(board_area.counter_area.position());
                p.line_to(iced::Point::new(
                    board_area.counter_area.x + board_area.counter_area.width,
                    board_area.counter_area.y,
                ));
                p.line_to(iced::Point::new(
                    board_area.counter_area.x + board_area.counter_area.width + counter_border_width,
                    board_area.counter_area.y - counter_border_width,
                ));
                p.line_to(iced::Point::new(
                    board_area.counter_area.x - counter_border_width,
                    board_area.counter_area.y - counter_border_width,
                ));
                p.line_to(iced::Point::new(
                    board_area.counter_area.x - counter_border_width,
                    board_area.counter_area.y + board_area.counter_area.height + counter_border_width,
                ));
                p.line_to(iced::Point::new(
                    board_area.counter_area.x,
                    board_area.counter_area.y + board_area.counter_area.height,
                ));
                p.close();
            });
            light_paths.push(counter_light_path);
            shadow_paths.push(counter_shadow_path);
        }

        Self {
            board_area,
            skin,
            borders,
            light_paths,
            shadow_paths,
            cache: canvas::Cache::new(),
        }
    }

    pub fn view(&self) -> iced::Element<'_, AppMessage> {
        canvas::Canvas::new(self)
            .width(self.board_area.canvas_area.width)
            .height(self.board_area.canvas_area.height)
            .into()
    }
}

impl canvas::Program<AppMessage> for BoardFrame {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<iced::Renderer>> {
        let geom = self.cache.draw(renderer, bounds.size(), |frame| {
            let background = canvas::Path::rectangle(iced::Point::ORIGIN, frame.size());
            frame.fill(&background, self.skin.background_color);
            if self.board_area.top_area != iced::Rectangle::default() {
                let top_area_background =
                    canvas::Path::rectangle(self.board_area.top_area.position(), self.board_area.top_area.size());
                frame.fill(&top_area_background, self.skin.top_area.background_color);
                if self.board_area.counter_area != iced::Rectangle::default() {
                    let counter_background = canvas::Path::rectangle(
                        self.board_area.counter_area.position(),
                        self.board_area.counter_area.size(),
                    );
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
        vec![geom]
    }
}
