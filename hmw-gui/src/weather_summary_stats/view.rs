use std::ops::RangeInclusive;

use hmw_data::HistogramStats;
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, rule, text},
};

use super::charts::HistogramBarChart;
use crate::{weather_summary_stats::charts::HistogramBarChartFlavor, widgets::follow_tooltip};

/// Displays histogram counters and year/day/time charts for one weather summary.
pub(crate) struct WeatherSummaryStats {
    charts: Vec<HistogramBarChart>,
}

impl WeatherSummaryStats {
    /// Builds the stats panel for a loaded weather summary.
    pub(crate) fn new(stats: HistogramStats<'_>, year_range: Option<RangeInclusive<i32>>) -> Self {
        let mut charts = Vec::with_capacity(3);
        charts.push(HistogramBarChart::new(
            stats.date_time,
            HistogramBarChartFlavor::Hod,
        ));
        charts.push(HistogramBarChart::new(
            stats.date_time,
            HistogramBarChartFlavor::Doy,
        ));
        if let Some(yr) = year_range { charts.push(HistogramBarChart::new(
            stats.date_time,
            HistogramBarChartFlavor::Year(yr),
        )) };
        Self { charts }
    }

    pub(crate) fn view<'a, Message: Clone + 'static>(
        &'a self,
        title: &'a str,
        stats: HistogramStats<'a>,
        on_back: Message,
    ) -> Element<'a, Message> {
        let mut skipped_breakdown = stats
            .histogram_counters
            .skipped
            .iter()
            .map(|(reason, count)| (reason.to_string(), *count))
            .collect::<Vec<_>>();
        skipped_breakdown.sort_by(|left, right| left.0.cmp(&right.0));

        let mut content = column([
            row([
                button("Back")
                    .padding([8, 14])
                    .style(button::secondary)
                    .on_press(on_back)
                    .into(),
                text(title).size(24).into(),
            ])
            .align_y(Alignment::Center)
            .spacing(16)
            .into(),
            counters_table(stats.histogram_counters.inserted, &skipped_breakdown).into(),
        ])
        .spacing(12)
        .width(Length::Fill);

        for chart in self.charts.iter() {
            let (title, missing) = match &chart.flavor {
                HistogramBarChartFlavor::Year(_) => {
                    ("Year Distribution", stats.date_time.counters.missing_year)
                }
                HistogramBarChartFlavor::Doy => (
                    "Day-of-Year Distribution",
                    stats.date_time.counters.missing_date,
                ),
                HistogramBarChartFlavor::Hod => (
                    "Hour-of-Day Distribution",
                    stats.date_time.counters.missing_time,
                ),
            };
            content = content.push(chart_title(title, missing));

            content = content.push(chart.view());
        }

        content.into()
    }
}

fn counters_table<Message: 'static>(
    inserted_count: usize,
    skipped_breakdown: &[(String, usize)],
) -> iced::widget::Column<'static, Message> {
    let total_skipped = skipped_breakdown
        .iter()
        .map(|(_, count)| *count)
        .sum::<usize>();

    let mut rows = Vec::new();

    if inserted_count != 0 {
        rows.push(counter_row(
            "In summary".to_string(),
            inserted_count,
            "Count of observations included in the summary.".to_string(),
            0,
            16,
        ));
    }

    if total_skipped != 0 {
        rows.push(counter_row(
            "Skipped".to_string(),
            total_skipped,
            "Observations matching years/month/location but are incomplete.".to_string(),
            0,
            16,
        ));

        for (reason, count) in skipped_breakdown.iter() {
            rows.push(counter_row(
                reason.clone(),
                *count,
                format!("Count of observations skipped because: {}", reason),
                20,
                14,
            ));
        }
    }

    rows.into_iter().enumerate().fold(
        column([]).spacing(0),
        |counters, (index, row)| match index {
            0 => counters.push(row),
            _ => counters.push(rule::horizontal(1)).push(row),
        },
    )
}

fn counter_row<Message: 'static>(
    label: String,
    value: usize,
    description: String,
    indent: u16,
    text_size: u16,
) -> Element<'static, Message> {
    follow_tooltip(
        container(
            row([
                container(text(""))
                    .width(Length::Fixed(indent as f32))
                    .into(),
                row([
                    text(label)
                        .size(text_size as f32)
                        .width(Length::FillPortion(3))
                        .into(),
                    // TODO: Human readable value based on locale.
                    text(value)
                        .size(text_size as f32)
                        .width(Length::FillPortion(1))
                        .into(),
                ])
                .width(Length::Fill)
                .align_y(Alignment::Center)
                .spacing(5)
                .into(),
            ])
            .align_y(Alignment::Center)
            .spacing(5),
        )
        .padding([5, 5])
        .style(container::transparent),
        text(description),
    )
}

fn chart_title<Message: 'static>(
    title: &'static str,
    missing_count: usize,
) -> Element<'static, Message> {
    let mut title_row = row([text(title).size(20).width(Length::Shrink).into()])
        .align_y(Alignment::End)
        .spacing(8);

    if missing_count != 0 {
        title_row = title_row.push(text(format!("(unknown: {} obs.)", missing_count)).size(13));
    }

    title_row.into()
}
