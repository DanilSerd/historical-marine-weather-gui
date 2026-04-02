use std::collections::HashMap;

use iced::{
    Alignment, Element, Font, Length,
    font::Weight,
    widget::{button, column, container, row, scrollable, text, toggler},
};

use crate::{
    collection::WeatherSummaryCollection,
    consts::ICON_FONT,
    types::{WeatherSummaryId, WeatherSummaryParams},
    weather_summary_stats::WeatherSummaryStats,
};

#[derive(Debug, Clone, Default)]
pub struct WeatherSummaryDetails {
    list: HashMap<WeatherSummaryId, WeatherSummaryControl>,
    selected_stats_summary: Option<WeatherSummaryId>,
}

#[derive(Debug, Clone)]
struct WeatherSummaryControl {
    visible: bool,
    show_points_on_map: bool,
}

#[derive(Debug, Clone)]
pub enum WeatherSummaryDetailsMessage {
    ToggleShowPointsOnMap(WeatherSummaryId, bool),
    OpenStats(WeatherSummaryId),
    CloseStats,
}

impl WeatherSummaryDetails {
    pub fn add(&mut self, id: WeatherSummaryId) {
        self.list.insert(id, id.into());
    }

    pub fn set_visible<'a>(&mut self, ids: impl IntoIterator<Item = &'a WeatherSummaryId>) {
        self.list.values_mut().for_each(|v| v.visible = false);
        ids.into_iter().for_each(|id| {
            if let Some(d) = self.list.get_mut(id) {
                d.visible = true;
            }
        });

        if self
            .selected_stats_summary
            .and_then(|id| self.list.get(&id))
            .map(|control| !control.visible)
            .unwrap_or(false)
        {
            self.selected_stats_summary = None;
        }
    }

    pub fn update(&mut self, message: WeatherSummaryDetailsMessage) {
        match message {
            WeatherSummaryDetailsMessage::ToggleShowPointsOnMap(id, selected) => {
                if let Some(control) = self.list.get_mut(&id) {
                    control.show_points_on_map = selected;
                }
            }
            WeatherSummaryDetailsMessage::OpenStats(id) => {
                self.selected_stats_summary = Some(id);
            }
            WeatherSummaryDetailsMessage::CloseStats => {
                self.selected_stats_summary = None;
            }
        }
    }

    pub fn all_show_points_on_map(&self) -> impl Iterator<Item = &WeatherSummaryId> {
        self.list.iter().filter_map(|(id, control)| {
            if control.show_points_on_map && control.visible {
                Some(id)
            } else {
                None
            }
        })
    }

    pub fn view<'a>(
        &'a self,
        collection: &'a WeatherSummaryCollection,
    ) -> Element<'a, WeatherSummaryDetailsMessage> {
        if let Some(selected_id) = self.selected_stats_summary
            && let Some(control) = self.list.get(&selected_id)
            && control.visible
            && let Some(summary) = collection.get(&selected_id)
            && let Some(stats) = summary.data.stats()
        {
            let year_range = summary
                .params
                .epoch
                .get_year_range()
                .map(|range| *range.start() as i32..=*range.end() as i32);

            return container(scrollable(WeatherSummaryStats::view(
                &summary.params.header.name,
                stats,
                year_range,
                WeatherSummaryDetailsMessage::CloseStats,
            )))
            .width(Length::Fill)
            .padding(10)
            .into();
        }

        let mut collection_iter: Vec<_> = self
            .list
            .iter()
            .filter(|(_, d)| d.visible)
            .filter_map(|(id, d)| collection.get(id).map(|summary| (summary, d)))
            .collect();

        collection_iter.sort_by_key(|(summary, _)| summary.params.header.id);

        let detail_iter = collection_iter.iter().map(|(summary, control)| {
            let header = text(&summary.params.header.name).size(18).font(Font {
                weight: Weight::Bold,
                ..Font::DEFAULT
            });

            let actions = row([
                row([
                    column([
                        text("Show on Globe").into(),
                        text(selected_cells_label(summary.params.geo.len()))
                            .size(13)
                            .into(),
                    ])
                    .spacing(2)
                    .into(),
                    toggler(control.show_points_on_map)
                        .on_toggle(|show| {
                            WeatherSummaryDetailsMessage::ToggleShowPointsOnMap(
                                summary.params.header.id,
                                show,
                            )
                        })
                        .into(),
                ])
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
                summary
                    .data
                    .stats()
                    .map(|_| {
                        button("View stats")
                            .style(button::secondary)
                            .on_press(WeatherSummaryDetailsMessage::OpenStats(
                                summary.params.header.id,
                            ))
                            .into()
                    })
                    .unwrap_or(text("No stats available").size(13).into()),
            ])
            .spacing(16)
            .align_y(Alignment::Center);

            let card = column([
                header.into(),
                params_to_element(&summary.params),
                actions.into(),
            ])
            .spacing(12)
            .width(Length::Fill);

            container(card)
                .padding(12)
                .width(Length::Fill)
                .style(style::card)
                .into()
        });

        if self.list.is_empty() {
            return container(text("Select weather summaries to see details."))
                .center(Length::Fill)
                .into();
        }

        let column = column(detail_iter).spacing(10);
        container(scrollable(column).width(Length::Fill))
            .width(Length::Fill)
            .padding(10)
            .into()
    }
}

impl From<WeatherSummaryId> for WeatherSummaryControl {
    fn from(_: WeatherSummaryId) -> Self {
        WeatherSummaryControl {
            visible: false,
            show_points_on_map: false,
        }
    }
}

fn params_to_element(params: &WeatherSummaryParams) -> Element<'_, WeatherSummaryDetailsMessage> {
    column([
        row([
            text("Type")
                .width(Length::Fixed(64.0))
                .font(Font {
                    weight: Weight::Bold,
                    ..Font::DEFAULT
                })
                .into(),
            row([
                text(params.header.summary_type.symbol())
                    .font(ICON_FONT)
                    .into(),
                text(params.header.summary_type.to_string()).into(),
            ])
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .into(),
        ])
        .spacing(12)
        .align_y(Alignment::Center)
        .into(),
        detail_row(
            "Months",
            params
                .months
                .iter()
                .map(|m| m.name())
                .collect::<Vec<_>>()
                .join(", "),
        ),
        detail_row("Years", params.epoch.to_string()),
    ])
    .spacing(8)
    .into()
}

fn detail_row<'a>(
    label: &'static str,
    value: impl Into<String>,
) -> Element<'a, WeatherSummaryDetailsMessage> {
    row([
        text(label)
            .width(Length::Fixed(64.0))
            .font(Font {
                weight: Weight::Bold,
                ..Font::DEFAULT
            })
            .into(),
        text(value.into()).width(Length::Fill).into(),
    ])
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn selected_cells_label(cells_count: usize) -> String {
    match cells_count {
        1 => "1 selected cell".to_string(),
        count => format!("{count} selected cells"),
    }
}

mod style {
    use iced::{Border, Theme, widget::container};

    pub fn card(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            border: Border {
                width: 2.0,
                color: palette.background.strong.color,
                ..Border::default()
            },
            ..container::Style::default()
        }
    }
}
