use crate::base::board::EncodeType;

#[derive(Clone, Debug)]
pub enum ErrorMessage {
    Acknowledge,
}

pub struct ErrorModal {
    pub error_message: String,
}

impl ErrorModal {
    pub fn new() -> Self {
        Self {
            error_message: String::new(),
        }
    }

    pub fn view(&self) -> iced::Element<'_, ErrorMessage> {
        iced::widget::container(
            iced::widget::column![
                iced::widget::center_x(
                    iced::widget::text("Error")
                        .color(iced::Color::from_rgb(1.0, 0.0, 0.0))
                        .size(20)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                ),
                iced::widget::center_x(iced::widget::text(&self.error_message).size(16)),
                iced::widget::center_x(
                    iced::widget::button(iced::widget::text("OK"))
                        .on_press(ErrorMessage::Acknowledge)
                        .padding([10, 20])
                ),
            ]
            .spacing(15),
        )
        .width(400)
        .padding(20)
        .style(iced::widget::container::rounded_box)
        .into()
    }
}
