use std::ops::RangeInclusive;

use iced::{
    Element, Length,
    widget::{column, container, rule, text},
};

use crate::loader::LoaderStats;
use crate::widgets::double_ended_slider;

#[derive(Debug, Clone)]
pub struct EpochSelector {
    pub epoch: Option<hmw_data::Epoch>,
    full_year_range: Option<RangeInclusive<u16>>,
}

#[derive(Debug, Clone)]
pub struct EpochSelectorMessage(hmw_data::Epoch);

impl EpochSelector {
    pub fn new(loader_stats: &LoaderStats, epoch: Option<hmw_data::Epoch>) -> Self {
        let epoch = epoch.or_else(|| {
            loader_stats
                .data_stats
                .as_ref()
                .map(|stats| hmw_data::Epoch::Range(stats.min_year..=stats.max_year))
        });
        Self {
            epoch,
            full_year_range: loader_stats
                .data_stats
                .as_ref()
                .map(|stats| stats.min_year..=stats.max_year),
        }
    }

    pub fn update(&mut self, message: EpochSelectorMessage) {
        self.epoch = Some(message.0);
    }

    pub fn view(&self) -> Element<'_, EpochSelectorMessage> {
        let (current_selected_start, current_selected_end, full_start, full_end) =
            match (self.epoch.as_ref(), self.full_year_range.as_ref()) {
                (Some(hmw_data::Epoch::Range(selected_range)), Some(full_range)) => (
                    *selected_range.start(),
                    *selected_range.end(),
                    *full_range.start(),
                    *full_range.end(),
                ),
                _ => return iced::widget::text("No observations").into(),
            };

        let selectors = double_ended_slider(
            full_start..=full_end,
            current_selected_start..=current_selected_end,
            |r| {
                let start = *r.start();
                let end = *r.end();

                EpochSelectorMessage(hmw_data::Epoch::Range(start..=end))
            },
        )
        .width(Length::Fill);

        let selectors: Element<'_, _> = selectors.into();
        let range_text = self.epoch.as_ref().unwrap().to_string();

        column([
            selectors,
            container(text(range_text)).center_x(Length::Fill).into(),
            rule::horizontal(1).into(),
        ])
        .spacing(6)
        .into()
    }

    pub fn is_empty(&self) -> bool {
        self.epoch.is_none()
    }
}
