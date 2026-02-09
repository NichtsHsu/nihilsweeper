use crate::ui::skin;
use log::{debug, trace};

#[derive(Debug, Clone, Copy, Default)]
pub struct BoardArea {
    pub canvas_area: iced::Rectangle,
    pub top_area: iced::Rectangle,
    pub game_area: iced::Rectangle,
    pub counter_area: iced::Rectangle,
    pub counter_digit_area: [iced::Rectangle; 3],
    pub face_area: iced::Rectangle,
}

impl BoardArea {
    pub fn calculate(skin: &skin::Skin, cell_size: u32, board_width: usize, board_height: usize) -> Self {
        let mut top_area = iced::Rectangle {
            x: skin.border.width,
            y: skin.border.width,
            width: board_width as f32 * cell_size as f32,
            height: skin.top_area.height,
        };

        let counter_offset = ((skin.top_area.height - skin.top_area.counter.height) / 2.0).floor();
        let counter_border_width = ((skin.top_area.counter.height
            - skin.top_area.counter.content_height
            - skin.top_area.counter.content_gap * 2.0)
            / 2.0)
            .floor();
        let counter_right_top;
        let mut counter_area = iced::Rectangle {
            x: top_area.x + counter_offset + counter_border_width,
            y: top_area.y + counter_offset + counter_border_width,
            width: skin.top_area.counter.content_width * 3.0 + skin.top_area.counter.content_gap * 6.0,
            height: skin.top_area.counter.content_height + skin.top_area.counter.content_gap * 2.0,
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
                    x: counter_area.x + skin.top_area.counter.content_gap,
                    y: counter_area.y + skin.top_area.counter.content_gap,
                    width: skin.top_area.counter.content_width,
                    height: skin.top_area.counter.content_height,
                },
                iced::Rectangle {
                    x: counter_area.x + skin.top_area.counter.content_width + skin.top_area.counter.content_gap * 3.0,
                    y: counter_area.y + skin.top_area.counter.content_gap,
                    width: skin.top_area.counter.content_width,
                    height: skin.top_area.counter.content_height,
                },
                iced::Rectangle {
                    x: counter_area.x
                        + skin.top_area.counter.content_width * 2.0
                        + skin.top_area.counter.content_gap * 5.0,
                    y: counter_area.y + skin.top_area.counter.content_gap,
                    width: skin.top_area.counter.content_width,
                    height: skin.top_area.counter.content_height,
                },
            ];
            counter_right_top = counter_area.x + counter_area.width + counter_border_width * 2.0;
            trace!("counter area: {:?}", counter_area);
            trace!("counter digit areas: {:?}", counter_digit_area);
        }

        let face_offset = ((skin.top_area.height - skin.top_area.face.size) / 2.0).floor();
        let face_area;
        if (counter_right_top + skin.top_area.face.size + face_offset * 2.0) > top_area.width {
            debug!("Not enough space for face, skipping");
            face_area = iced::Rectangle::default();
        } else if counter_right_top + skin.top_area.face.size / 2.0 + face_offset < top_area.x + top_area.width / 2.0 {
            debug!("Placing face in the center");
            face_area = iced::Rectangle {
                x: top_area.x + (top_area.width - skin.top_area.face.size) / 2.0,
                y: top_area.y + face_offset,
                width: skin.top_area.face.size,
                height: skin.top_area.face.size,
            };
        } else {
            debug!("Placing face to the right of counter");
            face_area = iced::Rectangle {
                x: counter_right_top + face_offset,
                y: top_area.y + face_offset,
                width: skin.top_area.face.size,
                height: skin.top_area.face.size,
            };
        }

        let game_area_offset = if counter_area == iced::Rectangle::default() && face_area == iced::Rectangle::default()
        {
            debug!("No top area, adjusting game area accordingly");
            top_area = iced::Rectangle::default();
            skin.border.width
        } else {
            top_area.height + skin.border.width * 2.0
        };
        let board_area = iced::Rectangle {
            x: skin.border.width,
            y: game_area_offset,
            width: board_width as f32 * cell_size as f32,
            height: board_height as f32 * cell_size as f32,
        };

        let game_area = iced::Rectangle {
            x: 0.0,
            y: 0.0,
            width: board_width as f32 * cell_size as f32 + skin.border.width * 2.0,
            height: board_area.y + board_area.height + skin.border.width,
        };

        BoardArea {
            canvas_area: game_area,
            top_area,
            game_area: board_area,
            counter_area,
            counter_digit_area,
            face_area,
        }
    }
}
