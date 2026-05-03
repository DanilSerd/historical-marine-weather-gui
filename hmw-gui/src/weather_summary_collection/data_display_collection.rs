use hmw_data::BeaufortScaleBucketer;
use iced::Element;

use crate::{
    collection::WeatherSummaryCollection,
    types::{WeatherSummary, WeatherSummaryId, WeatherSummaryKindEnum},
    windrose::{WindRose, WindRoseColorStrategy, WindRoseMessage},
};

#[derive(Debug, Clone)]
pub enum DataDisplayMessage {
    Wind(WindRoseMessage),
}

pub struct DataDisplay {
    wind: WindRose<BeaufortScaleBucketer>,
}

impl DataDisplay {
    pub fn new(windrose_color_strategy: WindRoseColorStrategy) -> Self {
        Self {
            wind: WindRose::new(windrose_color_strategy),
        }
    }
    pub fn insert(&mut self, summary: &WeatherSummary, visible: bool) {
        match (summary.data_avaialble(), summary) {
            (Ok(true), WeatherSummary::Wind(w)) => self.wind.insert(
                &summary.params().header.id,
                w.data().expect("data avaialble"),
                visible,
            ),
            (Ok(false), _) => (),
            (Err(_), _) => (),
        }
    }

    pub fn remove(&mut self, id: &WeatherSummaryId) {
        self.wind.remove(id);
    }

    pub fn set_visible<'a>(&mut self, ids: impl IntoIterator<Item = &'a WeatherSummaryId>) {
        self.wind.set_visible(ids);
    }

    pub fn update(&mut self, message: DataDisplayMessage, collection: &WeatherSummaryCollection) {
        match message {
            DataDisplayMessage::Wind(wrm) => self.wind.update(
                wrm,
                collection.iter().filter_map(|(id, s)| {
                    if let WeatherSummary::Wind(s) = s
                        && let Some(h) = s.data()
                    {
                        Some((id, h))
                    } else {
                        None
                    }
                }),
            ),
        };
    }

    pub fn view_data<'a>(
        &'a self,
        selected_type: WeatherSummaryKindEnum,
        collection: &'a WeatherSummaryCollection,
    ) -> (&'static str, Element<'a, DataDisplayMessage>) {
        match selected_type {
            WeatherSummaryKindEnum::Wind => (
                "Wind Rose",
                self.wind
                    .view_wind_rose(collection)
                    .map(DataDisplayMessage::Wind),
            ),
        }
    }

    pub fn view_sidepanel<'a>(
        &'a self,
        selected_type: WeatherSummaryKindEnum,
    ) -> (&'static str, Element<'a, DataDisplayMessage>) {
        match selected_type {
            WeatherSummaryKindEnum::Wind => (
                "Wind Key & Controls",
                self.wind.view_sidepanel().map(DataDisplayMessage::Wind),
            ),
        }
    }
}
