use std::sync::Arc;

use crate::widgets::tooltip_content_text;
use iced::advanced::{
    Layout, Widget,
    layout::{self, Node},
    mouse::Cursor,
    overlay,
    renderer::Style,
    widget::Tree,
    widget::tree,
};
use iced::{
    Color, Element, Font, Padding, Point, Renderer, Theme, Vector,
    widget::{Shader, Text},
};
use iced::{Length, Rectangle, Size};

use super::{RoseSector, WindRoseProgram, program::DEFAULT_GRIDLINE_COLOR};

const LABEL_TEXT_SIZE: f32 = 14.0;

pub struct WindRoseWidget {
    gridline_labels: Vec<Element<'static, (), Theme, Renderer>>,
    rose_sector_labels: Arc<Box<[String]>>,
    sectors: Arc<Box<[RoseSector]>>,
    instance: usize,
    scaling_factor: f32,
    apply_scaling_factor_to_gridlines: bool,
}

impl WindRoseWidget {
    pub fn new(
        instance: usize,
        sectors: Arc<Box<[RoseSector]>>,
        rose_sector_labels: Arc<Box<[String]>>,
        gridlines: u32,
        outer_probablity_label: f32,
        scaling_factor: f32,
        apply_scaling_factor_to_gridlines: bool,
    ) -> Self {
        let gridline_labels = (1..=gridlines)
            .map(|i| {
                let label_percentage = outer_probablity_label / gridlines as f32 * i as f32 * 100.;
                let label = if label_percentage < 0.001 {
                    "< 0.001%".to_string()
                } else if label_percentage <= 0.1 {
                    format!("{:.3}%", label_percentage)
                } else if label_percentage <= 1. {
                    format!("{:.2}%", label_percentage)
                } else {
                    format!("{:.1}%", label_percentage)
                };
                Text::new(label)
                    .size(LABEL_TEXT_SIZE)
                    .color(Color::from_rgba(
                        DEFAULT_GRIDLINE_COLOR.x,
                        DEFAULT_GRIDLINE_COLOR.y,
                        DEFAULT_GRIDLINE_COLOR.z,
                        1.,
                    ))
                    .font(Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .center()
                    .into()
            })
            .collect();
        Self {
            gridline_labels,
            rose_sector_labels,
            sectors,
            instance,
            scaling_factor,
            apply_scaling_factor_to_gridlines,
        }
    }

    fn new_shader(
        instance: usize,
        sectors: Arc<Box<[RoseSector]>>,
        gridlines: u32,
        highlight_segment: Option<u32>,
        scaling_factor: f32,
        apply_scaling_factor_to_gridlines: bool,
    ) -> Element<'static, (), Theme, Renderer> {
        let program = WindRoseProgram::new(
            instance,
            sectors,
            gridlines,
            highlight_segment,
            scaling_factor,
            apply_scaling_factor_to_gridlines,
        );
        Shader::new(program).into()
    }

    fn shader(&self, hovered_segment: Option<u32>) -> Element<'static, (), Theme, Renderer> {
        Self::new_shader(
            self.instance,
            self.sectors.clone(),
            self.gridline_labels.len() as u32,
            hovered_segment,
            self.scaling_factor,
            self.apply_scaling_factor_to_gridlines,
        )
    }

    fn tooltip(&self, hovered_segment: Option<u32>) -> Element<'static, (), Theme, Renderer> {
        tooltip_content_text(self.tooltip_text(hovered_segment))
    }

    fn tooltip_text(&self, hovered_segment: Option<u32>) -> &str {
        hovered_segment
            .and_then(|index| self.rose_sector_labels.get(index as usize))
            .map(String::as_str)
            .unwrap_or_default()
    }

    fn find_label(&self, cursor_point: Point, bounds: Rectangle) -> Option<u32> {
        let bounds_size: glam::Vec2 = <[f32; 2]>::from(bounds.size()).into();
        let vector_from_center: glam::Vec2 = glam::vec2(
            cursor_point.x - bounds.center_x(),
            bounds.center_y() - cursor_point.y,
        ) / bounds_size
            * 2.;
        let sector = self.sectors.iter().enumerate().find(|(_, s)| {
            let in_circle_slice = s.inner * self.scaling_factor <= vector_from_center.length()
                && vector_from_center.length() <= s.outer * self.scaling_factor;
            if !in_circle_slice {
                return false;
            }
            is_angle_between(vector_from_center, s.sweep_start_angle, s.sweep_end_angle)
        });

        sector.map(|(i, _)| i as u32)
    }
}

impl Widget<(), Theme, Renderer> for WindRoseWidget {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> Node {
        let limits_shrunk = limits.shrink(Padding::new(LABEL_TEXT_SIZE));
        let shader_size = if limits_shrunk.max().width > limits_shrunk.max().height {
            limits_shrunk.max().height
        } else {
            limits_shrunk.max().width
        };

        let mut shader_node = Node::new(Size::new(shader_size, shader_size));
        shader_node.align_mut(
            iced::Alignment::Center,
            iced::Alignment::Center,
            limits.max(),
        );

        let label_count = self.gridline_labels.len() as f32;
        let children_iter = self
            .gridline_labels
            .iter_mut()
            .enumerate()
            .map(|(i, label)| {
                let i = i + 1;
                let label_node =
                    label
                        .as_widget_mut()
                        .layout(&mut tree.children[i], renderer, limits);

                let mut y_translation = shader_node.size().height * 0.5 / label_count * i as f32;
                if self.apply_scaling_factor_to_gridlines {
                    y_translation *= self.scaling_factor;
                }
                label_node.translate(Vector::new(0., y_translation))
            });
        let mut children = vec![shader_node.clone()];
        children.extend(children_iter);

        Node::with_children(limits.max(), children)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let hovered_segment = tree.state.downcast_ref::<State>().hovered_segment;
        let shader = self.shader(hovered_segment);
        let iter = std::iter::once(&shader).chain(self.gridline_labels.iter());
        let iter = iter.zip(tree.children.iter()).zip(layout.children());
        for ((e, t), l) in iter {
            e.as_widget()
                .draw(t, renderer, theme, style, l, cursor, viewport);
        }
    }

    fn children(&self) -> Vec<Tree> {
        let shader = self.shader(None);
        let tooltip = self.tooltip(None);
        let mut children = vec![Tree::new(&shader)];
        children.extend(self.gridline_labels.iter().map(Tree::new));
        children.push(Tree::new(&tooltip));
        children
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        _event: &iced::Event,
        layout: Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, ()>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.children().next().unwrap().bounds();
        let position_in = cursor.position_in(bounds);
        let hovered_segment = position_in
            .and_then(|p| self.find_label(p, Rectangle::new(Point::ORIGIN, bounds.size())));

        let cursor_position = cursor
            .position()
            .zip(hovered_segment)
            .map(|(position, _)| position);

        if state.hovered_segment != hovered_segment || state.cursor_position != cursor_position {
            state.hovered_segment = hovered_segment;
            state.cursor_position = cursor_position;
            shell.invalidate_layout();
            shell.request_redraw();
        }
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        _layout: Layout<'a>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'a, (), Theme, Renderer>> {
        let state = tree.state.downcast_ref::<State>();
        let hovered_segment = state.hovered_segment?;
        let cursor_position = state.cursor_position?;

        Some(overlay::Element::new(Box::new(TooltipOverlay {
            tooltip: self.tooltip(Some(hovered_segment)),
            tree: tree.children.last_mut().expect("tooltip tree exists"),
            cursor_position,
        })))
    }

    fn diff(&self, tree: &mut Tree) {
        let hovered_segment = tree.state.downcast_ref::<State>().hovered_segment;
        let shader = self.shader(hovered_segment);
        let tooltip = self.tooltip(hovered_segment);
        let mut children = vec![shader.as_widget()];
        children.extend(self.gridline_labels.iter().map(|label| label.as_widget()));
        children.push(tooltip.as_widget());
        tree.diff_children(&children);
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct State {
    hovered_segment: Option<u32>,
    cursor_position: Option<Point>,
}

struct TooltipOverlay<'a> {
    tooltip: Element<'static, (), Theme, Renderer>,
    tree: &'a mut Tree,
    cursor_position: Point,
}

impl overlay::Overlay<(), Theme, Renderer> for TooltipOverlay<'_> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> Node {
        let viewport = Rectangle::with_size(bounds);
        let tooltip_node = self.tooltip.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        let tooltip_size = tooltip_node.size();
        let gap = 12.0;
        let max_x = (viewport.x + viewport.width - tooltip_size.width).max(viewport.x);
        let max_y = (viewport.y + viewport.height - tooltip_size.height).max(viewport.y);
        let x = (self.cursor_position.x + gap).clamp(viewport.x, max_x);
        let y = if self.cursor_position.y - tooltip_size.height - gap < viewport.y {
            self.cursor_position.y + gap
        } else {
            self.cursor_position.y - tooltip_size.height - gap
        }
        .clamp(viewport.y, max_y);

        Node::with_children(tooltip_size, vec![tooltip_node]).move_to(Point::new(x, y))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
    ) {
        self.tooltip.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout.children().next().expect("tooltip layout exists"),
            cursor,
            &Rectangle::with_size(Size::INFINITE),
        );
    }
}

impl From<WindRoseWidget> for Element<'static, (), Theme, Renderer> {
    fn from(rose: WindRoseWidget) -> Self {
        Self::new(rose)
    }
}

fn is_angle_between(vec: glam::Vec2, start_angle: f32, end_angle: f32) -> bool {
    let start_angle = start_angle % (std::f32::consts::PI * 2.0);
    let end_angle = end_angle % (std::f32::consts::PI * 2.0);

    if vec == glam::Vec2::ZERO {
        return true;
    }
    if start_angle == end_angle {
        return true;
    }
    let vec_angle = (vec.to_angle() + std::f32::consts::PI * 2.0) % (std::f32::consts::PI * 2.0);

    if start_angle <= end_angle {
        // Simple case: angle range doesn't cross 2π boundary
        vec_angle >= start_angle && vec_angle <= end_angle
    } else {
        // Complex case: angle range crosses 2π boundary
        vec_angle >= start_angle || vec_angle <= end_angle
    }
}
