use iced::widget::{button, column, container, row};
use iced::{Element, Length, Pixels};

use crate::utils::icon_widget;

pub struct Collapsible<'a, Message> {
    header: Element<'a, Message>,
    content: Element<'a, Message>,
    max_height: f32,
    is_expanded: bool,
    on_toggle: Box<dyn Fn(bool) -> Message + 'a>,
}

impl<'a, Message: Clone + 'a> Collapsible<'a, Message> {
    pub fn new(
        header: impl Into<Element<'a, Message>>,
        content: impl Into<Element<'a, Message>>,
        is_expanded: bool,
        on_toggle: impl Fn(bool) -> Message + 'a,
    ) -> Self {
        Self {
            header: header.into(),
            content: content.into(),
            is_expanded,
            on_toggle: Box::new(on_toggle),
            max_height: f32::INFINITY,
        }
    }

    /// Sets the maximum height.
    pub fn max_height(mut self, max_height: impl Into<Pixels>) -> Self {
        self.max_height = max_height.into().0;
        self
    }

    pub fn view(self) -> Element<'a, Message> {
        // Create arrow icon based on expanded state
        let arrow = if self.is_expanded {
            icon_widget("🔽") // Down arrow when expanded
        } else {
            icon_widget("▶") // Right arrow when collapsed
        };

        // Create header button with arrow and header content
        let header_content = row([arrow.into(), self.header])
            .spacing(8)
            .align_y(iced::Alignment::Center);

        let header_button = button(header_content)
            .on_press((self.on_toggle)(!self.is_expanded))
            .width(Length::Fill)
            .style(iced::widget::button::text);

        // Build the full widget
        let mut elements = vec![header_button.into()];

        if self.is_expanded {
            elements.push(
                container(self.content)
                    .padding(iced::Padding::from([0, 24])) // Left padding to indent content
                    .into(),
            );
        }

        container(column(elements).spacing(4))
            .max_height(self.max_height)
            .clip(true)
            .style(container::bordered_box)
            .into()
    }
}

impl<'a, Message: Clone + 'a> From<Collapsible<'a, Message>> for Element<'a, Message> {
    fn from(collapsible: Collapsible<'a, Message>) -> Self {
        collapsible.view()
    }
}
