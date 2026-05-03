use std::fmt::Display;

use crate::types::{WeatherSummaryId, WeatherSummaryKindEnum};

#[derive(Debug, Clone)]
pub enum WeatherListMessage {
    Delete(WeatherSummaryId),
    Checked(WeatherSummaryKindEnum, Option<WeatherSummaryId>, bool),
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

impl From<Result<bool, &str>> for WeatherListItemStatus {
    fn from(value: Result<bool, &str>) -> Self {
        match value {
            Ok(true) => WeatherListItemStatus::Loaded,
            Ok(false) => WeatherListItemStatus::Loading,
            Err(e) => WeatherListItemStatus::Error(e.to_owned()),
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
