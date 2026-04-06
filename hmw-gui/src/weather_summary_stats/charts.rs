use std::{hash::Hash, ops::RangeInclusive};

use chrono::{Datelike, NaiveDate};
use hmw_data::DateTimeHistogram;
use iced::{Element, Length};
use iced_aksel::{
    Axis, Chart, Plot, PlotData, PlotPoint, State,
    axis::{Position, TickContext, TickResult},
    scale::{Linear, Tick},
    shape::Rectangle,
};

const CHART_HEIGHT: f32 = 300.0;
const START_OF_LEAP_YEAR: NaiveDate = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
const X_AXIS_ID: &str = "x";
const Y_AXIS_ID: &str = "y";

pub(super) struct HistogramBarChart {
    pub(super) flavor: HistogramBarChartFlavor,
    state: State<&'static str, f64>,
    bars: iced_aksel::Cached<HistogramBars>,
}

impl HistogramBarChart {
    pub(super) fn new(histogram: &DateTimeHistogram, flavor: HistogramBarChartFlavor) -> Self {
        let human_formater = human_format::Formatter::new();
        let max = flavor.max_count(histogram) as f64;
        let state = State::new()
            .with_axis(
                X_AXIS_ID,
                Axis::new(flavor.x_domain(), Position::Bottom)
                    .with_thickness(42.0)
                    .skip_overlapping_labels(8.0)
                    .with_tick_renderer(flavor.x_tick_renderer()),
            )
            .with_axis(
                Y_AXIS_ID,
                Axis::new(Linear::new(0., max + max * 0.05), Position::Left)
                    .with_tick_renderer(move |ctx: TickContext<'_, f64>| {
                        let mut result = TickResult::default();
                        if ctx.tick.level == 0 {
                            result = result.grid_line(ctx.gridline());
                            if ctx.tick.value <= max {
                                result = result
                                    .label(ctx.label(human_formater.format(ctx.tick.value)))
                                    .tick_line(ctx.tickline());
                            }
                        }
                        result
                    })
                    .with_thickness(70)
                    .style(|style| {
                        // TODO: Fix upstream the spine rendering escaping the widget. Also affects
                        // the x axis.
                        style.spine.width = 0.0.into();
                    }),
            );
        let bars = iced_aksel::Cached::new(HistogramBars::from_histogram(histogram, &flavor));

        Self {
            bars,
            state,
            flavor,
        }
    }

    pub(super) fn view<Message: Clone + 'static>(&self) -> Element<'_, Message> {
        Chart::<_, _, Message, ()>::new(&self.state)
            .plot_data(&self.bars, X_AXIS_ID, Y_AXIS_ID)
            .width(Length::Fill)
            .height(Length::Fixed(CHART_HEIGHT))
            .into()
    }
}

struct HistogramBars {
    bins: Vec<HistogramBin>,
}

struct HistogramBin {
    start: f64,
    end: f64,
    count: f64,
}

impl HistogramBars {
    fn from_histogram(histogram: &DateTimeHistogram, flavor: &HistogramBarChartFlavor) -> Self {
        let bins = match flavor {
            HistogramBarChartFlavor::Year(year_range) => histogram
                .iter_year(year_range.clone())
                .map(|bucket| HistogramBin {
                    start: bucket.year as f64,
                    end: bucket.year as f64 + 1.,
                    count: bucket.count as f64,
                })
                .collect(),
            HistogramBarChartFlavor::Doy => histogram
                .iter_doy()
                .map(|bucket| HistogramBin {
                    start: bucket.day as f64,
                    end: bucket.day as f64 + 1.0,
                    count: bucket.count as f64,
                })
                .collect(),
            HistogramBarChartFlavor::Hod => histogram
                .iter_hod()
                .map(|bucket| HistogramBin {
                    start: bucket.hour as f64,
                    end: bucket.hour as f64 + 1.0,
                    count: bucket.count as f64,
                })
                .collect(),
        };

        Self { bins }
    }
}

impl<Message, Tag, Renderer> PlotData<f64, Message, Tag, Renderer> for HistogramBars
where
    Message: Clone,
    Tag: Hash + Eq + Clone,
    Renderer: iced_aksel::Renderer,
{
    fn draw(&self, plot: &mut Plot<'_, f64, Message, Tag, Renderer>, theme: &iced::Theme) {
        let bar_color = theme.extended_palette().primary.base.color;

        self.bins.iter().for_each(|bin| {
            plot.render(
                Rectangle::corners(
                    PlotPoint::new(bin.start, 0.0),
                    PlotPoint::new(bin.end, bin.count),
                )
                .fill(bar_color),
            );
        });
    }
}

#[derive(Clone)]
pub(super) enum HistogramBarChartFlavor {
    Year(RangeInclusive<i32>),
    Doy,
    Hod,
}

impl HistogramBarChartFlavor {
    fn x_domain(&self) -> Linear<f64, f32> {
        match self {
            HistogramBarChartFlavor::Year(r) => {
                let r = r.clone();
                let years = r.end() - r.start();
                let step_by = if years >= 100 {
                    10
                } else if years >= 50 {
                    5
                } else if years >= 20 {
                    2
                } else {
                    1
                };
                Linear::new_with_tick_fn(*r.start() as f64, *r.end() as f64 + 1., move |_| {
                    r.clone()
                        .step_by(step_by)
                        .map(|year| Tick {
                            value: year as f64,
                            level: 0,
                        })
                        .collect()
                })
            }
            HistogramBarChartFlavor::Doy => Linear::new_with_tick_fn(0.0, 366.0, |_| {
                (1..=12)
                    .map(|month| Tick {
                        value: NaiveDate::from_ymd_opt(2000, month, 1).unwrap().ordinal0() as f64,
                        level: 0,
                    })
                    .collect()
            }),
            HistogramBarChartFlavor::Hod => Linear::new_with_tick_fn(0.0, 24.0, |_| {
                (0..24)
                    .map(|hour| Tick {
                        value: hour as f64,
                        level: 0,
                    })
                    .collect()
            }),
        }
    }

    fn x_tick_renderer(&self) -> impl FnMut(TickContext<'_, f64>) -> TickResult + use<> {
        match self {
            HistogramBarChartFlavor::Doy => |ctx: TickContext<'_, f64>| {
                let date = START_OF_LEAP_YEAR
                    .checked_add_days(chrono::Days::new(ctx.tick.value.floor() as u64))
                    .unwrap();
                let month = chrono::Month::try_from(u8::try_from(date.month()).unwrap()).unwrap();
                TickResult::default()
                    .tick_line(ctx.tickline())
                    .grid_line(ctx.gridline())
                    .label(ctx.label(format!("{} {}", &month.name()[0..3], date.day())))
            },
            HistogramBarChartFlavor::Year(_) => |ctx: TickContext<'_, f64>| {
                TickResult::default()
                    .tick_line(ctx.tickline())
                    .grid_line(ctx.gridline())
                    .label(ctx.label((ctx.tick.value as i32).to_string()))
            },
            HistogramBarChartFlavor::Hod => |ctx: TickContext<'_, f64>| {
                TickResult::default()
                    .tick_line(ctx.tickline())
                    .grid_line(ctx.gridline())
                    .label(ctx.label(format!("{:02}:00", ctx.tick.value as u32)))
            },
        }
    }

    fn max_count(&self, histogram: &DateTimeHistogram) -> usize {
        match self {
            HistogramBarChartFlavor::Year(range) => histogram.max_year_count(range.clone()).max(1),
            HistogramBarChartFlavor::Doy => histogram.max_doy_count().max(1),
            HistogramBarChartFlavor::Hod => histogram.max_hod_count().max(1),
        }
    }
}
