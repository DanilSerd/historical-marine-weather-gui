use iced::{
    Element, Length,
    widget::{Id, Space, column, text, text::LineHeight},
};

#[derive(Debug, Clone)]
pub struct NameSelector {
    pub id: Id,
    pub name: String,
    pub name_not_allowed: bool,
}

#[derive(Debug, Clone)]
pub struct NameSelectorMessage {
    pub name: String,
}

impl NameSelector {
    pub fn new(name: String) -> Self {
        Self {
            name,
            id: Id::unique(),
            name_not_allowed: false,
        }
    }

    pub fn update<'a>(
        &mut self,
        message: NameSelectorMessage,
        mut existing_names: impl Iterator<Item = &'a str>,
    ) {
        let duplicate_name = existing_names.any(|name| name == message.name);
        self.name = message.name;
        self.name_not_allowed = duplicate_name;
    }

    pub fn view(&self) -> Element<'_, NameSelectorMessage> {
        let mut text_input =
            iced::widget::text_input("My windrose...", self.name.as_str()).id(self.id.clone());
        if self.name_not_allowed {
            text_input = text_input.style(|theme: &iced::Theme, status| {
                let palette = theme.extended_palette();
                let mut default = iced::widget::text_input::default(theme, status);
                default.border.color = palette.danger.base.color;
                default
            });
        }
        text_input = text_input.on_input(|name| NameSelectorMessage { name });

        let error: Element<'_, NameSelectorMessage> = if self.name_not_allowed {
            text("This name already exists")
                .size(10.)
                .line_height(LineHeight::Relative(1.))
                .style(iced::widget::text::danger)
                .into()
        } else {
            Space::new()
                .width(Length::Fill)
                .height(Length::Fixed(10.))
                .into()
        };

        column([text_input.into(), error]).into()
    }

    pub fn is_ready(&self) -> bool {
        !self.name.is_empty() && !self.name_not_allowed
    }
}
