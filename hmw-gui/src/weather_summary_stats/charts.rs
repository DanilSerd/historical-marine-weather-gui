use std::ops::RangeInclusive;

use chrono::{Days, NaiveDate};
use hmw_data::DateTimeHistogram;
use iced::{Element, Length};
use plotters::{
    coord::{Shift, ranged1d::IntoSegmentedCoord},
    prelude::*,
};
use plotters_iced2::{Chart, ChartBuilder, ChartWidget, DrawingBackend};

use super::doy_coord::DayOfYearly;

const CHART_HEIGHT: f32 = 300.0;
const START_OF_LEAP_YEAR: NaiveDate = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
const START_OF_NEXT_YEAR: NaiveDate = NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
const Y_LABEL_AREA_PADDING: u32 = 4;
const DEFAULT_Y_LABEL_AREA_SIZE: u32 = 32;

const BAR_STYLE: ShapeStyle = ShapeStyle {
    color: RGBAColor(36, 92, 135, 1.),
    filled: true,
    stroke_width: 0,
};

pub(super) enum HistogramBarChartFlavor {
    Year(RangeInclusive<i32>),
    Doy,
    Hod,
}

pub(super) struct HistogramBarChart<'a> {
    histogram: &'a DateTimeHistogram,
    flavor: HistogramBarChartFlavor,
}

impl<'a> HistogramBarChart<'a> {
    pub(super) fn new(histogram: &'a DateTimeHistogram, flavor: HistogramBarChartFlavor) -> Self {
        Self { histogram, flavor }
    }

    pub(super) fn view<Message: 'static>(self) -> Element<'a, Message> {
        ChartWidget::new(self)
            .width(Length::Fill)
            .height(Length::Fixed(CHART_HEIGHT))
            .into()
    }

    fn configure_builder<DB: DrawingBackend>(
        &self,
        chart_builder: &mut ChartBuilder<DB>,
        left_label_area_size: u32,
    ) {
        chart_builder
            .margin_top(8)
            .margin_right(16)
            .margin_bottom(16)
            .set_label_area_size(LabelAreaPosition::Left, left_label_area_size)
            .set_label_area_size(LabelAreaPosition::Bottom, 42);
    }

    fn build_doy_chart<DB: DrawingBackend>(
        &self,
        mut chart_builder: ChartBuilder<DB>,
        pixels_width: u32,
        left_label_area_size: u32,
    ) {
        self.configure_builder(&mut chart_builder, left_label_area_size);

        let mut chart = chart_builder
            .build_cartesian_2d(
                DayOfYearly::from(START_OF_LEAP_YEAR..START_OF_NEXT_YEAR).into_segmented(),
                0usize..self.histogram.max_doy_count(),
            )
            .unwrap();

        let x_labels = ((pixels_width / 90) as usize).clamp(2, 24);
        let formatter = human_format::Formatter::new();
        let _ = chart
            .configure_mesh()
            .label_style(("sans-serif", 12))
            .x_labels(x_labels)
            .y_label_formatter(&|value| formatter.format(*value as f64))
            .draw();

        let _ = chart.draw_series(
            Histogram::vertical(&chart)
                .data(self.histogram.iter_doy().map(|bucket| {
                    (
                        START_OF_LEAP_YEAR
                            .checked_add_days(Days::new(bucket.day as u64))
                            .unwrap(),
                        bucket.count,
                    )
                }))
                .margin(0)
                .style(BAR_STYLE),
        );
    }

    fn build_hod_chart<DB: DrawingBackend>(
        &self,
        mut chart_builder: ChartBuilder<DB>,
        pixels_width: u32,
        left_label_area_size: u32,
    ) {
        self.configure_builder(&mut chart_builder, left_label_area_size);

        let mut chart = chart_builder
            .build_cartesian_2d(0u32..24, 0usize..self.histogram.max_hod_count())
            .unwrap();

        let x_labels = ((pixels_width / 90) as usize).clamp(4, 24);
        let formatter = human_format::Formatter::new();
        let _ = chart
            .configure_mesh()
            .label_style(("sans-serif", 12))
            .x_label_formatter(&|value| format_hod_label(*value))
            .x_labels(x_labels)
            .y_label_formatter(&|value| formatter.format(*value as f64))
            .draw();

        let _ = chart.draw_series(
            Histogram::vertical(&chart)
                .data(
                    self.histogram
                        .iter_hod()
                        .map(|bucket| (bucket.hour, bucket.count)),
                )
                .margin(0)
                .style(BAR_STYLE),
        );
    }

    fn build_year_chart<DB: DrawingBackend>(
        &self,
        mut chart_builder: ChartBuilder<DB>,
        pixels_width: u32,
        year_range: RangeInclusive<i32>,
        left_label_area_size: u32,
    ) {
        self.configure_builder(&mut chart_builder, left_label_area_size);

        let year_start = *year_range.start();
        let year_end = *year_range.end() + 1;
        let mut chart = chart_builder
            .build_cartesian_2d(
                year_start..year_end,
                0usize..self.histogram.max_year_count(year_range.clone()),
            )
            .unwrap();

        let x_labels = ((pixels_width / 90) as usize).clamp(2, 12);
        let formatter = human_format::Formatter::new();
        let _ = chart
            .configure_mesh()
            .label_style(("sans-serif", 12))
            .x_label_formatter(&|value| value.to_string())
            .x_labels(x_labels)
            .y_label_formatter(&|value| formatter.format(*value as f64))
            .draw();

        let _ = chart.draw_series(
            Histogram::vertical(&chart)
                .data(
                    self.histogram
                        .iter_year(year_range.clone())
                        .map(|bucket| (bucket.year, bucket.count)),
                )
                .margin(0)
                .style(BAR_STYLE),
        );
    }
}

impl<'a, Message> Chart<Message> for HistogramBarChart<'a> {
    type State = ();

    fn build_chart<DB: DrawingBackend>(
        &self,
        _state: &Self::State,
        _chart_builder: ChartBuilder<DB>,
    ) {
        unimplemented!("draw_chart is overriden so this is not used");
    }

    fn draw_chart<DB: DrawingBackend>(&self, _state: &Self::State, root: DrawingArea<DB, Shift>) {
        let (width, _) = root.dim_in_pixel();
        let left_label_area_size = y_axis_label_area_size(&root);

        let builder = ChartBuilder::on(&root);

        match &self.flavor {
            HistogramBarChartFlavor::Year(range) => {
                self.build_year_chart(builder, width, range.clone(), left_label_area_size)
            }
            HistogramBarChartFlavor::Doy => {
                self.build_doy_chart(builder, width, left_label_area_size)
            }
            HistogramBarChartFlavor::Hod => {
                self.build_hod_chart(builder, width, left_label_area_size)
            }
        }
    }
}

fn format_hod_label(value: u32) -> String {
    format!("{:02}:00", value)
}

fn y_axis_label_area_size<DB: DrawingBackend>(root: &DrawingArea<DB, Shift>) -> u32 {
    let widest_label = "000.00 k";
    let label_style = ("sans-serif", 14).into_text_style(root);

    root.estimate_text_size(widest_label, &label_style)
        .map(|(width, _)| width + Y_LABEL_AREA_PADDING)
        .unwrap_or(DEFAULT_Y_LABEL_AREA_SIZE)
}
