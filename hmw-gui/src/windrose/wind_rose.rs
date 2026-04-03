use std::collections::HashMap;
use std::f32;
use std::ops::RangeInclusive;
use std::sync::Arc;
use std::{cmp::Ordering, fmt::Display};

use hmw_data::{CardinalOrdinalDirection, DirectionalBucketing, DirectionalIntensityHistogram};
use iced::{
    Background, Color, Element, Font, Length, Theme,
    alignment::Horizontal,
    widget::{Space, checkbox, column, container, row, rule, text},
};

use crate::collection::WeatherSummaryCollection;
use crate::types::WeatherSummaryId;
use crate::widgets::{DoubleEndedSliderStyle, double_ended_slider};

use super::{
    rendering::{RoseSector, WindRoseWidget},
    utils::fit_square_elements_to_scrollable,
};

const DEFAULT_GRIDLINES: u32 = 5;
const MIN_SQUARE_SIZE: f32 = 400.;

#[derive(Debug, Clone)]
pub enum WindRoseMessage {
    LimitSpeedBuckets(RangeInclusive<usize>),
    ToggleDirectionalGrid(bool),
    ToggleUniformScaling(bool),
    None,
}

pub struct WindRose<B: DirectionalBucketing> {
    data: HashMap<WeatherSummaryId, WindRoseDisplayData>,
    color_map: ColorMap<B::BinInterval>,
    all_buckets: Vec<B::BinInterval>,
    intensity_buckets_limits: RangeInclusive<usize>,
    directional_grid: bool,
    uniform_scaling: bool,
}

impl<B> WindRose<B>
where
    B: DirectionalBucketing + Send + Sync + 'static,
    B::BinInterval: Eq + std::hash::Hash + Copy + Ord + Display + std::fmt::Debug,
{
    pub fn new(color_strategy: WindRoseColorStrategy) -> Self {
        let mut all_buckets = B::default().bins().collect::<Vec<_>>();
        all_buckets.sort();
        let wind_speed_buckets_limits = 0..=all_buckets.len() - 1;

        Self {
            data: HashMap::new(),
            color_map: color_strategy.color_map(all_buckets.iter().copied()),
            all_buckets,
            intensity_buckets_limits: wind_speed_buckets_limits,
            directional_grid: true,
            uniform_scaling: true,
        }
    }

    pub fn insert(
        &mut self,
        id: &WeatherSummaryId,
        histogram: &DirectionalIntensityHistogram<B>,
        visible: bool,
    ) {
        self.data.insert(
            *id,
            from_histogram_to_wind_display_data(
                histogram,
                id,
                visible,
                &self.color_map,
                self.get_buckets_limited(),
            ),
        );
    }

    pub fn update<'a>(
        &mut self,
        message: WindRoseMessage,
        collection: impl IntoIterator<
            Item = (&'a WeatherSummaryId, &'a DirectionalIntensityHistogram<B>),
        >,
    ) {
        match message {
            WindRoseMessage::LimitSpeedBuckets(range_inclusive) => {
                self.update_bucket_limits(range_inclusive);
                let v: Vec<_> = collection
                    .into_iter()
                    .filter_map(|(id, histogram)| {
                        self.data.get(id).map(|d| (id, histogram, d.visible))
                    })
                    .collect();
                v.into_iter().for_each(|(id, histogram, visible)| {
                    self.insert(id, histogram, visible);
                });
            }
            WindRoseMessage::ToggleDirectionalGrid(directional) => {
                self.directional_grid = directional;
            }
            WindRoseMessage::ToggleUniformScaling(uniform) => {
                self.uniform_scaling = uniform;
            }
            WindRoseMessage::None => panic!("WindRoseMessage::None should not be sent"),
        }
    }

    pub fn remove(&mut self, id: &WeatherSummaryId) {
        self.data.remove(id);
    }

    pub fn set_visible<'a>(&mut self, ids: impl IntoIterator<Item = &'a WeatherSummaryId>) {
        self.data.values_mut().for_each(|v| v.visible = false);
        ids.into_iter().for_each(|id| {
            if let Some(d) = self.data.get_mut(id) {
                d.visible = true;
            }
        });
    }

    pub fn view_wind_rose<'a>(
        &'a self,
        collection: &'a WeatherSummaryCollection,
    ) -> Element<'a, WindRoseMessage> {
        iced::widget::responsive(move |size| {
            let roses = self.view_windrose_widgets(collection);
            let n = roses.len();
            fit_square_elements_to_scrollable(roses, n, size, MIN_SQUARE_SIZE).into()
        })
        .into()
    }

    pub fn view_sidepanel(&self) -> Element<'_, WindRoseMessage> {
        let buckets_limited = self.get_buckets_limited();
        let key: Element<'_, _> = column(self.color_map.iter_ordered().map(|(s, c)| {
            let (key_color, font) = if buckets_limited.contains(s) {
                (
                    Color::from_linear_rgba(c.x, c.y, c.z, c.w),
                    Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    },
                )
            } else {
                (
                    Color::from_linear_rgba(0.5, 0.5, 0.5, 1.0),
                    Font {
                        weight: iced::font::Weight::Thin,
                        ..Default::default()
                    },
                )
            };
            let key_color_container: Element<'_, _> =
                container(Space::new().width(Length::Fill).height(Length::Fill))
                    .style(move |_: &Theme| {
                        iced::widget::container::background(Background::Color(key_color))
                    })
                    .width(Length::FillPortion(2))
                    .height(Length::Fill)
                    .max_height(20)
                    .into();

            let key_description: Element<'_, _> = text(s.to_string())
                .width(Length::FillPortion(8))
                .height(Length::Fill)
                .center()
                .size(12)
                .font(font)
                .into();
            container(row([key_color_container, key_description]).padding(2))
                .max_height(20)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }))
        .into();

        let num_visible_roses = self.all_visible_wind_roses().count();
        let speed_selection_sliders = self.view_wind_speed_selectors();
        let gridline_style_checkbox: Element<'_, bool> = container(
            checkbox(self.directional_grid)
                .label("Directional grid")
                .on_toggle(|t| t),
        )
        .center_x(Length::Shrink)
        .into();

        let mut scaled_checkbox = checkbox(self.uniform_scaling).label("Scaled");
        if num_visible_roses > 1 {
            scaled_checkbox = scaled_checkbox.on_toggle(|t| t);
        }
        let uniform_scaling_checkbox: Element<'_, bool> =
            container(scaled_checkbox).center_x(Length::Shrink).into();

        let side_panel = column([
            key,
            speed_selection_sliders,
            gridline_style_checkbox.map(WindRoseMessage::ToggleDirectionalGrid),
            uniform_scaling_checkbox.map(WindRoseMessage::ToggleUniformScaling),
        ])
        .spacing(8);
        container(side_panel)
            .padding(5)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn update_bucket_limits(&mut self, mut new_limits: RangeInclusive<usize>) {
        if new_limits.start() > new_limits.end() {
            new_limits = *new_limits.end()..=*new_limits.start();
        }
        if new_limits.end() < new_limits.start() {
            new_limits = *new_limits.start()..=*new_limits.end();
        }
        self.intensity_buckets_limits = new_limits;
    }

    fn get_buckets_limited(&self) -> &[B::BinInterval] {
        &self.all_buckets[self.intensity_buckets_limits.clone()]
    }

    fn all_visible_wind_roses(&self) -> impl Iterator<Item = &WindRoseDisplayData> {
        self.data.iter().filter(|(_, v)| v.visible).map(|(_, d)| d)
    }

    fn visible_max_directional_probability(&self) -> f32 {
        self.all_visible_wind_roses()
            .map(|d| d.max_direction_probability)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap_or(1.)
    }

    fn visible_max_sum_probability(&self) -> f32 {
        self.all_visible_wind_roses()
            .map(|d| d.sum_direction_probabilities)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap_or(1.)
    }

    fn view_windrose_widgets<'a>(
        &'a self,
        collection: &'a WeatherSummaryCollection,
    ) -> Vec<Element<'a, WindRoseMessage>> {
        let mut sorted: Vec<_> = self.all_visible_wind_roses().collect::<Vec<_>>();
        sorted.sort_by_key(|d| d.id);
        let iter = sorted
            .into_iter()
            .filter_map(|d| collection.get(&d.id).map(|h| (d, &h.params.header)));

        let max_directional_probablity = self.visible_max_directional_probability();
        let max_sum_probability = self.visible_max_sum_probability();

        let iter = iter.enumerate().map(move |(i, (data, header))| {
            let (outer_prob_label, gridlines, scale_factor, apply_scaling_factor_to_gridlines) =
                match (self.directional_grid, self.uniform_scaling) {
                    (true, true) => (
                        max_directional_probablity,
                        DEFAULT_GRIDLINES,
                        data.max_direction_probability / max_directional_probablity,
                        false,
                    ),
                    (false, true) => (
                        data.sum_direction_probabilities,
                        1,
                        data.sum_direction_probabilities / max_sum_probability,
                        true,
                    ),
                    (true, false) => (data.max_direction_probability, DEFAULT_GRIDLINES, 1., false),
                    (false, false) => (data.sum_direction_probabilities, 1, 1., false),
                };

            let widget: Element<'static, ()> = WindRoseWidget::new(
                i,
                data.sectors.clone(),
                data.rose_sector_labels.clone(),
                gridlines,
                outer_prob_label,
                scale_factor,
                apply_scaling_factor_to_gridlines,
            )
            .into();
            let widget: Element<'_, WindRoseMessage> = widget.map(|_| WindRoseMessage::None);
            let name: Element<'_, WindRoseMessage> = text(header.name.as_str())
                .align_x(Horizontal::Center)
                .width(Length::Fill)
                .into();
            container(column([name, widget]))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|theme| {
                    let color_palette = theme.extended_palette();
                    iced::widget::container::rounded_box(theme)
                        .background(color_palette.background.base.color)
                })
                .padding(5)
                .into()
        });
        iter.collect()
    }

    fn view_wind_speed_selectors(&self) -> Element<'_, WindRoseMessage> {
        let mut colors_iter = self
            .color_map
            .iter_ordered()
            .enumerate()
            .filter(|(i, _)| self.intensity_buckets_limits.contains(i))
            .map(|(_, (_, c))| c);
        let lower_color = colors_iter.next().unwrap();
        let upper_color = colors_iter.last().unwrap_or(lower_color);

        let slider: Element<'_, _> = double_ended_slider(
            0.0..=((self.all_buckets.len() - 1) as f64),
            *self.intensity_buckets_limits.start() as f64
                ..=*self.intensity_buckets_limits.end() as f64,
            |value| {
                WindRoseMessage::LimitSpeedBuckets(*value.start() as usize..=*value.end() as usize)
            },
        )
        .width(Length::Fill)
        .step(1.0)
        .style(DoubleEndedSliderStyle::new(
            Color::from_linear_rgba(lower_color.x, lower_color.y, lower_color.z, lower_color.w),
            Color::from_linear_rgba(upper_color.x, upper_color.y, upper_color.z, upper_color.w),
        ))
        .into();

        column([
            row([text("Filter:").into(), slider]).spacing(8).into(),
            rule::horizontal(1).into(),
        ])
        .spacing(6)
        .into()
    }
}

struct WindRoseDisplayData {
    id: WeatherSummaryId,
    visible: bool,
    sectors: Arc<Box<[RoseSector]>>,
    rose_sector_labels: Arc<Box<[String]>>,
    max_direction_probability: f32,
    sum_direction_probabilities: f32,
}

impl WindRoseDisplayData {
    fn empty(id: WeatherSummaryId, visible: bool) -> Self {
        Self {
            id,
            visible,
            sectors: Arc::new(Box::new([])),
            rose_sector_labels: Arc::new(Box::new([])),
            max_direction_probability: 0.,
            sum_direction_probabilities: 0.,
        }
    }
}

fn from_histogram_to_wind_display_data<B: DirectionalBucketing>(
    histogram: &DirectionalIntensityHistogram<B>,
    id: &WeatherSummaryId,
    visible: bool,
    color_map: &ColorMap<B::BinInterval>,
    buckets_limited: &[B::BinInterval],
) -> WindRoseDisplayData
where
    B::BinInterval: Display + PartialEq + Ord + std::hash::Hash + Copy + std::fmt::Debug,
{
    let mut indeterminate_bucket_prob = 0.;
    let mut indeterminate_bucket_intensity: Option<B::BinInterval> = None;
    let mut histo_buckets = histogram
        .iter_non_empty()
        .filter(|b| buckets_limited.contains(&b.intensity_bucket))
        .filter_map(|b| {
            if b.direction_bucket == CardinalOrdinalDirection::Indeterminate {
                debug_assert!(
                    indeterminate_bucket_intensity.is_none(),
                    "indeterminate bucket intensity already set. We should not have more than one indeterminate bucket."
                );
                indeterminate_bucket_prob += b.probability;
                indeterminate_bucket_intensity = Some(b.intensity_bucket);
                return None;
            }
            Some(b)
        })
        .collect::<Vec<_>>();

    if histo_buckets.is_empty() {
        if let Some(indeterminate_bucket_intensity) = indeterminate_bucket_intensity {
            return WindRoseDisplayData {
                id: *id,
                visible,
                sectors: Arc::new(Box::new([RoseSector::new(
                    color_map
                        .get_color(&indeterminate_bucket_intensity)
                        .unwrap(),
                    0.,
                    1.,
                    SectorAngleRange::from(CardinalOrdinalDirection::Indeterminate).0,
                    SectorAngleRange::from(CardinalOrdinalDirection::Indeterminate).1,
                )
                .expect("correct indeterminate sector")])),
                rose_sector_labels: Arc::new(Box::new([format!(
                    "{}: {:.3}%",
                    indeterminate_bucket_intensity,
                    indeterminate_bucket_prob * 100.
                )])),
                max_direction_probability: indeterminate_bucket_prob as f32,
                sum_direction_probabilities: indeterminate_bucket_prob as f32,
            };
        }
        return WindRoseDisplayData::empty(*id, visible);
    }

    let indeterminate_bucket_prob_per_direction = indeterminate_bucket_prob / 8.;

    histo_buckets.sort_by_key(|b| (b.direction_bucket, b.intensity_bucket));

    let mut current_direction_bucket = histo_buckets[0].direction_bucket;

    let mut directional_probabilities = [indeterminate_bucket_prob_per_direction; 8];
    let mut i = 0;

    for b in histo_buckets.iter() {
        if b.direction_bucket != current_direction_bucket {
            current_direction_bucket = b.direction_bucket;
            i += 1;
        }
        directional_probabilities[i] += b.probability;
    }

    let sum_dir_probs: f64 = directional_probabilities.iter().sum();
    let max_dir_prob = *directional_probabilities
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
        .unwrap();

    let mut sectors = Vec::with_capacity(8 * buckets_limited.len() + 1);
    let mut rose_sector_labels = Vec::with_capacity(8 * buckets_limited.len() + 1);

    match (
        indeterminate_bucket_prob_per_direction,
        indeterminate_bucket_intensity,
    ) {
        (p, Some(i)) if p > 0. => {
            let sector_angle_range =
                SectorAngleRange::from(CardinalOrdinalDirection::Indeterminate);
            sectors.push(
                RoseSector::new(
                    color_map.get_color(&i).expect("color for bucket"),
                    0.,
                    (p / max_dir_prob) as f32,
                    sector_angle_range.0,
                    sector_angle_range.1,
                )
                .expect("correct indeterminate sector"),
            );
            rose_sector_labels.push(format!("{}: {:.3}%", i, indeterminate_bucket_prob * 100.));
        }
        _ => (),
    }

    let mut current_direction_bucket = histo_buckets[0].direction_bucket;
    let mut sector_angle_range = SectorAngleRange::from(current_direction_bucket);
    let mut sector_inner = indeterminate_bucket_prob_per_direction / max_dir_prob;

    for b in histo_buckets.iter() {
        if b.direction_bucket != current_direction_bucket {
            current_direction_bucket = b.direction_bucket;
            sector_angle_range = SectorAngleRange::from(current_direction_bucket);
            sector_inner = indeterminate_bucket_prob_per_direction / max_dir_prob;
        }
        let sector_outer = sector_inner + b.probability / max_dir_prob;
        let sector = RoseSector::new(
            color_map
                .get_color(&b.intensity_bucket)
                .expect("color for bucket"),
            sector_inner as f32,
            sector_outer as f32,
            sector_angle_range.0,
            sector_angle_range.1,
        )
        .expect("correct sector");

        sector_inner = sector_outer;
        sectors.push(sector);
        rose_sector_labels.push(format!(
            "{},{}: {:.3}%",
            current_direction_bucket,
            b.intensity_bucket,
            b.probability * 100.
        ));
    }

    WindRoseDisplayData {
        id: *id,
        visible,
        sectors: Arc::new(sectors.into_boxed_slice()),
        rose_sector_labels: Arc::new(rose_sector_labels.into_boxed_slice()),
        max_direction_probability: max_dir_prob as f32,
        sum_direction_probabilities: sum_dir_probs as f32,
    }
}

#[derive(Debug, Clone)]
pub enum WindRoseColorStrategy {
    /// (start lerp colour, middle lerp colour, end lerp colour)
    Lerp(glam::Vec4, glam::Vec4, glam::Vec4),
}

impl WindRoseColorStrategy {
    fn color_map<B>(&self, buckets: impl IntoIterator<Item = B>) -> ColorMap<B>
    where
        B: Eq + std::hash::Hash + Copy,
    {
        let map = match self {
            WindRoseColorStrategy::Lerp(start, middle, end) => {
                let all_buckets = buckets.into_iter().collect::<Vec<_>>();

                all_buckets
                    .iter()
                    .enumerate()
                    .map(|(i, buck)| {
                        let t = i as f32 / (all_buckets.len() - 1) as f32;
                        let color = if t <= 0.5 {
                            start.lerp(*middle, t * 2.0)
                        } else {
                            middle.lerp(*end, (t - 0.5) * 2.0)
                        };
                        (*buck, color)
                    })
                    .collect::<HashMap<_, _>>()
            }
        };
        ColorMap(map)
    }
}

impl Default for WindRoseColorStrategy {
    fn default() -> Self {
        Self::Lerp(
            glam::Vec4::new(0., 0., 1., 1.),
            glam::Vec4::new(0.2, 1., 0., 1.),
            glam::Vec4::new(1., 0., 0., 1.),
        )
    }
}

#[derive(Debug, Clone)]
struct ColorMap<K>(HashMap<K, glam::Vec4>);

impl<K> ColorMap<K>
where
    K: Ord + std::hash::Hash,
{
    pub fn iter_ordered(&self) -> impl Iterator<Item = (&K, &glam::Vec4)> {
        let mut keys = self.0.keys().collect::<Vec<_>>();
        keys.sort();
        keys.into_iter().map(move |k| (k, self.0.get(k).unwrap()))
    }

    pub fn get_color(&self, key: &K) -> Option<glam::Vec4> {
        self.0.get(key).copied()
    }
}

const POSSIBLE_WIND_DIRECTIONS_IN_ORDER_OF_ROSE_SECTORS: [CardinalOrdinalDirection; 8] = [
    CardinalOrdinalDirection::E,
    CardinalOrdinalDirection::NE,
    CardinalOrdinalDirection::N,
    CardinalOrdinalDirection::NW,
    CardinalOrdinalDirection::W,
    CardinalOrdinalDirection::SW,
    CardinalOrdinalDirection::S,
    CardinalOrdinalDirection::SE,
];
const SWEEP_ANGLE_PORTION_RATIO: f32 = 0.9;

struct SectorAngleRange(f32, f32);

impl From<CardinalOrdinalDirection> for SectorAngleRange {
    fn from(value: CardinalOrdinalDirection) -> Self {
        if value == CardinalOrdinalDirection::Indeterminate {
            return SectorAngleRange(0., f32::consts::PI * 2.);
        }

        let id = POSSIBLE_WIND_DIRECTIONS_IN_ORDER_OF_ROSE_SECTORS
            .iter()
            .position(|d| d == &value)
            .expect("all directions are mapped");
        let sp = f32::consts::PI * 2. + id as f32 * f32::consts::FRAC_PI_4;
        let (sweep_start_angle, sweep_end_angle) = (
            sp - f32::consts::FRAC_PI_8 * SWEEP_ANGLE_PORTION_RATIO,
            sp + f32::consts::FRAC_PI_8 * SWEEP_ANGLE_PORTION_RATIO,
        );
        SectorAngleRange(sweep_start_angle, sweep_end_angle)
    }
}

impl<B> std::fmt::Debug for WindRose<B>
where
    B: DirectionalBucketing + std::fmt::Debug,
    B::BinInterval: std::fmt::Debug + std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindRose")
            .field("data", &"..")
            .field("color_map", &self.color_map)
            .finish()
    }
}
