use iced::Element;

use crate::types::WeatherSummaryKindEnum;

#[derive(Debug, Clone)]
pub struct WeatherTypeSelector {
    pub weather_type: WeatherSummaryKindEnum,
}

#[derive(Debug, Clone)]
pub struct WeatherTypeSelectorMessage {
    pub weather_type: WeatherSummaryKindEnum,
}

impl WeatherTypeSelector {
    pub fn new() -> Self {
        Self {
            weather_type: Default::default(),
        }
    }

    pub fn view(&self) -> Element<'_, WeatherTypeSelectorMessage> {
        let picker = iced::widget::pick_list(
            WeatherSummaryKindEnum::all_kinds().collect::<Vec<_>>(),
            Some(self.weather_type),
            |weather_type| WeatherTypeSelectorMessage { weather_type },
        );
        picker.into()
    }
}
