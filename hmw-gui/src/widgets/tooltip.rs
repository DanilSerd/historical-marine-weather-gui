use iced::{
    Element,
    widget::{container, tooltip},
};

/// Wraps content with the app's shared follow-cursor tooltip styling.
pub fn follow_tooltip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    content_tooltip: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    tooltip(
        content.into(),
        tooltip_content(content_tooltip),
        tooltip::Position::FollowCursor,
    )
    .into()
}

fn tooltip_content<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(content.into())
        .padding(6)
        .style(container::bordered_box)
        .into()
}
