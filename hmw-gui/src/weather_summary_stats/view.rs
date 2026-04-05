use std::ops::RangeInclusive;

use hmw_data::HistogramStats;
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, rule, text},
};

use super::charts::HistogramBarChart;
use crate::{weather_summary_stats::charts::HistogramBarChartFlavor, widgets::follow_tooltip};

/// Displays histogram counters and year/day/time charts for one weather summary.
pub(crate) struct WeatherSummaryStats;

impl WeatherSummaryStats {
    /// Builds the stats panel for a loaded weather summary.
    pub(crate) fn view<'a, Message: Clone + 'static>(
        title: &'a str,
        stats: HistogramStats<'a>,
        year_range: Option<RangeInclusive<i32>>,
        on_back: Message,
    ) -> Element<'a, Message> {
        let mut c = column([
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
            counters_table(&stats).into(),
            chart_title(
                "Hour-of-Day Distribution",
                stats.date_time.counters.missing_time,
            ),
            chart_section(HistogramBarChart::new(
                stats.date_time,
                HistogramBarChartFlavor::Hod,
            ))
            .into(),
            chart_title(
                "Day-of-Year Distribution",
                stats.date_time.counters.missing_date,
            ),
            chart_section(HistogramBarChart::new(
                stats.date_time,
                HistogramBarChartFlavor::Doy,
            ))
            .into(),
        ])
        .spacing(12)
        .width(Length::Fill);

        if let Some(yr) = year_range {
            let yd = chart_section(HistogramBarChart::new(
                stats.date_time,
                HistogramBarChartFlavor::Year(yr),
            ));
            c = c.push(chart_title(
                "Year Distribution",
                stats.date_time.counters.missing_year,
            ));
            c = c.push(<Element<'a, Message>>::from(yd));
        }
        c.into()
    }
}

fn counters_table<'a, Message: 'static>(
    stats: &HistogramStats<'a>,
) -> iced::widget::Column<'a, Message> {
    let histogram = stats.histogram_counters;
    let total_skipped = histogram.skipped.values().copied().sum::<usize>();
    let mut skipped_breakdown = histogram.skipped.iter().collect::<Vec<_>>();
    skipped_breakdown.sort_by_key(|(reason, _)| **reason);

    let mut rows = Vec::new();

    if histogram.inserted != 0 {
        rows.push(counter_row(
            "In summary".to_string(),
            histogram.inserted,
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

        for (reason, count) in skipped_breakdown {
            rows.push(counter_row(
                reason.to_string(),
                *count,
                format!("Count of observations skipped because: {}", reason),
                20,
                14,
            ));
        }
    }

    let mut counters = column([]).spacing(0);

    for (index, row) in rows.into_iter().enumerate() {
        if index != 0 {
            counters = counters.push(rule::horizontal(1));
        }

        counters = counters.push(row);
    }

    counters
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
                    // TODO: human readable count based on locale
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

fn chart_section<'a, Message: 'static>(
    chart: HistogramBarChart<'a>,
) -> iced::widget::Column<'a, Message> {
    column([chart.view()]).spacing(8)
}
