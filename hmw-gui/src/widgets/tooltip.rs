use iced::{
    Element,
    widget::{container, text, tooltip},
};

/// Builds tooltip contents using the app's shared tooltip styling.
pub(crate) fn tooltip_content<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(content.into())
        .padding(6)
        .style(container::bordered_box)
        .into()
}

/// Builds text tooltip contents using the app's shared tooltip styling.
pub(crate) fn tooltip_content_text<'a, Message: 'a>(
    tooltip_text: impl Into<String>,
) -> Element<'a, Message> {
    tooltip_content(text(tooltip_text.into()))
}

/// Wraps content with the app's shared follow-cursor tooltip styling.
pub(crate) fn follow_tooltip<'a, Message: 'a>(
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

/// Wraps content with a text tooltip using the app's shared styling.
pub(crate) fn follow_tooltip_text<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    tooltip_text: impl Into<String>,
) -> Element<'a, Message> {
    follow_tooltip(content, tooltip_content_text(tooltip_text))
}
