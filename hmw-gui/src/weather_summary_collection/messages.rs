use iced::widget::pane_grid;

use crate::{
    earth_map::EarthMapMessage,
    types::WeatherSummary,
    weather_summary_collection::data_display_collection::DataDisplayMessage,
    weather_summary_details::WeatherSummaryDetailsMessage,
    weather_summary_form::{FormSubmitted, NewWeatherSummaryFormMessage},
    weather_summary_list::WeatherListMessage,
};

#[derive(Debug, Clone, Default)]
pub enum WeatherSummaryCollectionMessage {
    EarthMapMessage(EarthMapMessage),
    WeatherListMessage(WeatherListMessage),
    DataDisplayMessage(DataDisplayMessage),
    NewWeatherSummaryFormMessage(NewWeatherSummaryFormMessage),
    WeatherSummaryFormSubmitted(FormSubmitted),
    SummaryLoaded(WeatherSummary),
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
    CloseDataDisplay,
    #[default]
    None,
}
