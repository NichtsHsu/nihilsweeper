use crate::{
    base::{Vec2D, board},
    config::GlobalConfig,
    ui::skin,
};
use iced::widget::canvas;
use log::{debug, trace};

#[derive(Debug, Clone)]
pub struct EditorMessage {}

pub struct Editor {
    board: Vec2D<board::CellState>,
    mines: usize,
    editor_area: iced::Rectangle,
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

impl Editor {
    pub fn new(config: &GlobalConfig, skin: skin::Skin) -> Self {
        let mut this = Self {
            board: Vec2D::new(config.board[0], config.board[1]),
            mines: 0,
            editor_area: iced::Rectangle::default(),
            top_area: iced::Rectangle::default(),
            board_area: iced::Rectangle::default(),
            counter_area: iced::Rectangle::default(),
            counter_digit_area: [iced::Rectangle::default(); 3],
            face_area: iced::Rectangle::default(),
            borders: Vec::new(),
            light_paths: Vec::new(),
            shadow_paths: Vec::new(),
            cell_size: config.cell_size,
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
            width: self.board.dims().0 as f32 * self.cell_size as f32,
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
            width: self.board.dims().0 as f32 * self.cell_size as f32,
            height: self.board.dims().1 as f32 * self.cell_size as f32,
        };

        let game_area = iced::Rectangle {
            x: 0.0,
            y: 0.0,
            width: self.board.dims().0 as f32 * self.cell_size as f32 + self.skin.border.width * 2.0,
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

        self.editor_area = game_area;
        self.top_area = top_area;
        self.board_area = board_area;
        self.counter_area = counter_area;
        self.counter_digit_area = counter_digit_area;
        self.face_area = face_area;
        self.borders = borders;
        self.light_paths = light_paths;
        self.shadow_paths = shadow_paths;
    }

    pub fn update(&mut self) {}

    pub fn view(&self) -> iced::Element<'_, EditorMessage> {
        iced::widget::scrollable("todo").into()
    }
}
