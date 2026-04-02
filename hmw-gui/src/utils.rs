use iced::widget::{Row, Text, button, row, text};
use iced::{Alignment, Element};

use crate::{consts::ICON_FONT, widgets::follow_tooltip_text};

pub fn icon_widget(icon: &str) -> Text<'_> {
    iced::widget::text(icon).font(ICON_FONT)
}

/// Formats a count with comma separators for display in the GUI.
pub(crate) fn format_count(value: usize) -> String {
    value
        .to_string()
        .chars()
        .rev()
        .enumerate()
        .flat_map(|(index, ch)| {
            (index > 0 && index % 3 == 0)
                .then_some(',')
                .into_iter()
                .chain(std::iter::once(ch))
        })
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

/// Shared control bar icon identifiers.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ControlBarIcon {
    Data,
    NewCollection,
    OpenCollection,
    SaveCollection,
    SaveCollectionAs,
    NewSummary,
    ViewSummaries,
    ViewMain,
}

/// Returns the emoji used for a control bar action.
pub(crate) const fn control_bar_icon(icon: ControlBarIcon) -> &'static str {
    match icon {
        ControlBarIcon::Data => "🗄",
        ControlBarIcon::NewCollection => "✨",
        ControlBarIcon::OpenCollection => "📂",
        ControlBarIcon::SaveCollection => "💾",
        ControlBarIcon::SaveCollectionAs => "📄",
        ControlBarIcon::NewSummary => "➕",
        ControlBarIcon::ViewSummaries => "👁",
        ControlBarIcon::ViewMain => "👁",
    }
}

/// Builds compact icon and label content for a button.
pub(crate) fn icon_label<'a, Message: 'a>(icon: &'a str, label: &'a str) -> Row<'a, Message> {
    row([icon_widget(icon).into(), text(label).into()])
        .spacing(6)
        .align_y(Alignment::Center)
}

/// Builds a button with a follow-cursor tooltip.
pub(crate) fn tooltip_button<'a, Message: Clone + 'a>(
    icon: &'a str,
    label: &'a str,
    full_name: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let button = match on_press {
        Some(message) => button(icon_label(icon, label)).on_press(message),
        None => button(icon_label(icon, label)),
    };

    follow_tooltip_text(button, full_name)
}
