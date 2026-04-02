use iced::Element;

use crate::types::WeatherSummaryType;

#[derive(Debug, Clone)]
pub struct WeatherTypeSelector {
    pub weather_type: WeatherSummaryType,
}

#[derive(Debug, Clone)]
pub struct WeatherTypeSelectorMessage {
    pub weather_type: WeatherSummaryType,
}

impl WeatherTypeSelector {
    pub fn new() -> Self {
        Self {
            weather_type: WeatherSummaryType::Wind,
        }
    }

    pub fn view(&self) -> Element<'_, WeatherTypeSelectorMessage> {
        let picker = iced::widget::pick_list(
            WeatherSummaryType::all_types(),
            Some(self.weather_type),
            |weather_type| WeatherTypeSelectorMessage { weather_type },
        );
        picker.into()
    }
}
