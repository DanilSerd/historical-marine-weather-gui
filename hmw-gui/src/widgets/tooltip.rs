use iced::{
    Element,
    widget::{container, text, tooltip},
};

/// Wraps content with the app's shared follow-cursor tooltip styling.
pub(crate) fn follow_tooltip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    tooltip_content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    tooltip(
        content.into(),
        tooltip_content.into(),
        tooltip::Position::FollowCursor,
    )
    .padding(6)
    .style(container::bordered_box)
    .into()
}

/// Wraps content with a text tooltip using the app's shared styling.
pub(crate) fn follow_tooltip_text<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    tooltip_text: impl Into<String>,
) -> Element<'a, Message> {
    follow_tooltip(content, text(tooltip_text.into()))
}
