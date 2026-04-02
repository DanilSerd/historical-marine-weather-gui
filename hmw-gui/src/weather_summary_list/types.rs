use std::fmt::Display;

use crate::types::{WeatherSummaryData, WeatherSummaryId, WeatherSummaryType};

#[derive(Debug, Clone)]
pub enum WeatherListMessage {
    Delete(WeatherSummaryId),
    Checked(WeatherSummaryType, Option<WeatherSummaryId>, bool),
    Hovered(WeatherSummaryId, bool),
    Edit(WeatherSummaryId),
    Duplicate(WeatherSummaryId),
    New,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum WeatherListItemStatus {
    #[default]
    Loading,
    Loaded,
    Error(String),
}

impl From<&WeatherSummaryData> for WeatherListItemStatus {
    fn from(value: &WeatherSummaryData) -> Self {
        match value {
            WeatherSummaryData::Error(e) => WeatherListItemStatus::Error(e.clone()),
            WeatherSummaryData::None => WeatherListItemStatus::Loading,
            _ => WeatherListItemStatus::Loaded,
        }
    }
}

pub struct WeatherListItem {
    pub selected: bool,
    pub hovered: bool,
}

impl WeatherListItem {
    pub fn new() -> Self {
        Self {
            selected: false,
            hovered: false,
        }
    }
}

pub const ALL_LIST_ITEM_CONTROL_OPTIONS: [WeatherListItemControlOption; 3] = [
    WeatherListItemControlOption::Delete,
    WeatherListItemControlOption::Edit,
    WeatherListItemControlOption::Duplicate,
];

#[derive(Debug, Clone, PartialEq)]
pub enum WeatherListItemControlOption {
    Delete,
    Edit,
    Duplicate,
}

impl WeatherListItemControlOption {
    pub fn to_message(&self, id: WeatherSummaryId) -> WeatherListMessage {
        match self {
            WeatherListItemControlOption::Delete => WeatherListMessage::Delete(id),
            WeatherListItemControlOption::Edit => WeatherListMessage::Edit(id),
            WeatherListItemControlOption::Duplicate => WeatherListMessage::Duplicate(id),
        }
    }
}

impl Display for WeatherListItemControlOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            WeatherListItemControlOption::Delete => "🗑",
            WeatherListItemControlOption::Edit => "✏︎",
            WeatherListItemControlOption::Duplicate => "📄📄",
        };
        f.write_str(s)
    }
}
