use iced::widget::pane_grid;

use crate::{
    earth_map::EarthMapMessage,
    types::{WeatherSummaryData, WeatherSummaryId, WeatherSummaryType},
    weather_summary_details::WeatherSummaryDetailsMessage,
    weather_summary_form::{FormSubmitted, NewWeatherSummaryFormMessage},
    weather_summary_list::WeatherListMessage,
    windrose::WindRoseMessage,
};

#[derive(Debug, Clone, Default)]
pub enum WeatherSummaryCollectionMessage {
    EarthMapMessage(EarthMapMessage),
    WeatherListMessage(WeatherListMessage),
    WindRoseMessage(WindRoseMessage, WeatherSummaryType),
    NewWeatherSummaryFormMessage(NewWeatherSummaryFormMessage),
    WeatherSummaryFormSubmitted(FormSubmitted),
    SummaryLoaded(WeatherSummaryId, WeatherSummaryData),
    PaneMessage(PaneMessage),
    ControlBarMessage(ControlBarMessage),
    WeatherSummaryDetailsMessage(WeatherSummaryDetailsMessage),
    #[default]
    None,
}

#[derive(Debug, Clone, Copy)]
pub enum ControlBarMessage {
    ToggleScreen,
    NewForm,
}

#[derive(Debug, Clone, Default)]
pub enum PaneMessage {
    Resized(pane_grid::ResizeEvent),
    CloseForm,
    CloseWindRose,
    #[default]
    None,
}
