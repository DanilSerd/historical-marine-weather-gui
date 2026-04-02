use std::{marker::PhantomData, ops::RangeInclusive};

use iced::{
    Background, Color, Element, Event, Length, Point, Radians, Rectangle, Size, Theme,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, renderer,
        widget::tree::{self, Tree},
    },
    border::Border,
    gradient, touch,
    widget::slider,
};

const DEFAULT_HEIGHT: f32 = 16.0;
const MERGED_DRAG_SPLIT_THRESHOLD: f32 = 2.0;

/// Numeric values supported by the double-ended slider.
pub(crate) trait RangeValue: Copy + PartialOrd {
    fn one() -> Self;
    fn into_f64(self) -> f64;
    fn from_f64(value: f64) -> Self;
}

impl RangeValue for f64 {
    fn one() -> Self {
        1.0
    }

    fn into_f64(self) -> f64 {
        self
    }

    fn from_f64(value: f64) -> Self {
        value
    }
}

impl RangeValue for i32 {
    fn one() -> Self {
        1
    }

    fn into_f64(self) -> f64 {
        f64::from(self)
    }

    fn from_f64(value: f64) -> Self {
        value as i32
    }
}

impl RangeValue for usize {
    fn one() -> Self {
        1
    }

    fn into_f64(self) -> f64 {
        self as f64
    }

    fn from_f64(value: f64) -> Self {
        value as usize
    }
}

impl RangeValue for u16 {
    fn one() -> Self {
        1
    }

    fn into_f64(self) -> f64 {
        self as f64
    }

    fn from_f64(value: f64) -> Self {
        value as u16
    }
}

/// Styling for a double-ended slider.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DoubleEndedSliderStyle {
    /// The color of the start handle.
    pub start_handle_color: Color,
    /// The color of the end handle.
    pub end_handle_color: Color,
}

impl DoubleEndedSliderStyle {
    /// Creates a new [`DoubleEndedSliderStyle`].
    pub(crate) fn new(start_handle_color: Color, end_handle_color: Color) -> Self {
        Self {
            start_handle_color,
            end_handle_color,
        }
    }
}

/// Creates a new double-ended slider.
pub(crate) fn double_ended_slider<'a, T, Message, F>(
    range: RangeInclusive<T>,
    selection: RangeInclusive<T>,
    on_change: F,
) -> DoubleEndedSlider<T, Message, F>
where
    T: RangeValue,
    F: 'a + Fn(RangeInclusive<T>) -> Message,
{
    DoubleEndedSlider::new(range, selection, on_change)
}

/// A horizontal slider with independently draggable start and end handles.
pub(crate) struct DoubleEndedSlider<T, Message, F>
where
    F: Fn(RangeInclusive<T>) -> Message,
{
    range: RangeInclusive<T>,
    selection: RangeInclusive<T>,
    on_change: F,
    step: T,
    width: Length,
    height: f32,
    style: DoubleEndedSliderStyle,
    marker: PhantomData<Message>,
}

impl<T, Message, F> DoubleEndedSlider<T, Message, F>
where
    T: RangeValue,
    F: Fn(RangeInclusive<T>) -> Message,
{
    fn new(range: RangeInclusive<T>, selection: RangeInclusive<T>, on_change: F) -> Self {
        Self {
            range: range.clone(),
            selection: clamp_selection(&range, selection),
            on_change,
            step: T::one(),
            width: Length::Fill,
            height: DEFAULT_HEIGHT,
            style: DoubleEndedSliderStyle::new(Color::BLACK, Color::BLACK),
            marker: PhantomData,
        }
    }

    /// Sets the width of the slider.
    pub(crate) fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the step size of the slider.
    pub(crate) fn step(mut self, step: impl Into<T>) -> Self {
        self.step = step.into();
        self
    }

    /// Sets the style of the slider.
    pub(crate) fn style(mut self, style: DoubleEndedSliderStyle) -> Self {
        self.style = style;
        self
    }

    fn handle_radius(&self) -> f32 {
        self.height / 2.0
    }
}

impl<T, Message, Renderer, F> Widget<Message, Theme, Renderer> for DoubleEndedSlider<T, Message, F>
where
    T: RangeValue,
    Renderer: renderer::Renderer,
    F: Fn(RangeInclusive<T>) -> Message,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Fixed(self.height),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.width, self.height)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                let Some(cursor_position) = cursor.position_over(bounds) else {
                    return;
                };

                state.drag_state = pressed_drag_state(
                    cursor_position,
                    bounds,
                    &self.range,
                    &self.selection,
                    self.handle_radius(),
                );

                match state.drag_state {
                    DragState::Start => update_selection(
                        self,
                        shell,
                        state,
                        ActiveHandle::Start,
                        locate_value(bounds, cursor_position, &self.range, self.step),
                    ),
                    DragState::End => update_selection(
                        self,
                        shell,
                        state,
                        ActiveHandle::End,
                        locate_value(bounds, cursor_position, &self.range, self.step),
                    ),
                    DragState::Idle | DragState::PendingMerged { .. } => {}
                }

                shell.capture_event();
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Touch(touch::Event::FingerMoved { .. }) => {
                let Some(cursor_position) = cursor.position() else {
                    return;
                };

                match state.drag_state {
                    DragState::Idle => {}
                    DragState::Start => {
                        update_selection(
                            self,
                            shell,
                            state,
                            ActiveHandle::Start,
                            locate_value(bounds, cursor_position, &self.range, self.step),
                        );
                        shell.capture_event();
                    }
                    DragState::End => {
                        update_selection(
                            self,
                            shell,
                            state,
                            ActiveHandle::End,
                            locate_value(bounds, cursor_position, &self.range, self.step),
                        );
                        shell.capture_event();
                    }
                    DragState::PendingMerged { origin_x } => {
                        let delta_x = cursor_position.x - origin_x;

                        if delta_x.abs() < MERGED_DRAG_SPLIT_THRESHOLD {
                            return;
                        }

                        let active_handle = match delta_x.is_sign_negative() {
                            true => ActiveHandle::Start,
                            false => ActiveHandle::End,
                        };

                        state.drag_state = drag_state(active_handle);

                        update_selection(
                            self,
                            shell,
                            state,
                            active_handle,
                            locate_value(bounds, cursor_position, &self.range, self.step),
                        );
                        shell.capture_event();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. })
            | Event::Touch(touch::Event::FingerLost { .. }) => {
                state.drag_state = DragState::Idle;
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let default_style = slider::default(theme, slider::Status::Active);
        let state = tree.state.downcast_ref::<State>();
        let handle_radius = self.handle_radius();
        let rail_bounds = rail_bounds(bounds, default_style.rail.width, handle_radius);
        let start_center =
            handle_center(bounds, *self.selection.start(), &self.range, handle_radius);
        let end_center = handle_center(bounds, *self.selection.end(), &self.range, handle_radius);

        renderer.fill_quad(
            renderer::Quad {
                bounds: rail_bounds,
                border: default_style.rail.border,
                ..renderer::Quad::default()
            },
            default_style.rail.backgrounds.1,
        );

        let fill_x = start_center.x.min(end_center.x);
        let fill_width = (start_center.x - end_center.x).abs();

        if fill_width > 0.0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: fill_x,
                        y: rail_bounds.y,
                        width: fill_width,
                        height: rail_bounds.height,
                    },
                    border: Border::default(),
                    ..renderer::Quad::default()
                },
                Background::from(
                    gradient::Linear::new(Radians::PI / 2.0)
                        .add_stop(0.0, self.style.start_handle_color)
                        .add_stop(1.0, self.style.end_handle_color),
                ),
            );
        }

        let handle_border = Border {
            radius: handle_radius.into(),
            width: default_style.handle.border_width,
            color: default_style.handle.border_color,
        };

        if selection_is_merged(&self.selection) {
            let merged_color = match state.merged_handle {
                ActiveHandle::Start => self.style.start_handle_color,
                ActiveHandle::End => self.style.end_handle_color,
            };

            draw_handle(
                renderer,
                start_center,
                handle_radius,
                handle_border,
                merged_color,
            );
        } else {
            draw_handle(
                renderer,
                start_center,
                handle_radius,
                handle_border,
                self.style.start_handle_color,
            );
            draw_handle(
                renderer,
                end_center,
                handle_radius,
                handle_border,
                self.style.end_handle_color,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();

        if state.drag_state == DragState::Idle && !cursor.is_over(layout.bounds()) {
            mouse::Interaction::default()
        } else if cfg!(target_os = "windows") {
            mouse::Interaction::Pointer
        } else if state.drag_state == DragState::Idle {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::Grabbing
        }
    }
}

impl<'a, T, Message, Renderer, F> From<DoubleEndedSlider<T, Message, F>>
    for Element<'a, Message, Theme, Renderer>
where
    T: RangeValue + 'a,
    Message: 'a,
    Renderer: renderer::Renderer + 'a,
    F: Fn(RangeInclusive<T>) -> Message + 'a,
{
    fn from(slider: DoubleEndedSlider<T, Message, F>) -> Self {
        Element::new(slider)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveHandle {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DragState {
    Idle,
    Start,
    End,
    PendingMerged { origin_x: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct State {
    drag_state: DragState,
    merged_handle: ActiveHandle,
}

impl Default for State {
    fn default() -> Self {
        Self {
            drag_state: DragState::Idle,
            merged_handle: ActiveHandle::End,
        }
    }
}

fn clamp_selection<T>(range: &RangeInclusive<T>, selection: RangeInclusive<T>) -> RangeInclusive<T>
where
    T: RangeValue,
{
    let mut start = clamp_value(*selection.start(), range);
    let mut end = clamp_value(*selection.end(), range);

    if start > end {
        std::mem::swap(&mut start, &mut end);
    }

    start..=end
}

fn clamp_value<T>(value: T, range: &RangeInclusive<T>) -> T
where
    T: RangeValue,
{
    if value < *range.start() {
        *range.start()
    } else if value > *range.end() {
        *range.end()
    } else {
        value
    }
}

fn drag_state(handle: ActiveHandle) -> DragState {
    match handle {
        ActiveHandle::Start => DragState::Start,
        ActiveHandle::End => DragState::End,
    }
}

fn pressed_drag_state<T>(
    cursor_position: Point,
    bounds: Rectangle,
    range: &RangeInclusive<T>,
    selection: &RangeInclusive<T>,
    handle_radius: f32,
) -> DragState
where
    T: RangeValue,
{
    let start_center = handle_center(bounds, *selection.start(), range, handle_radius);
    let end_center = handle_center(bounds, *selection.end(), range, handle_radius);
    let on_start = point_is_on_handle(cursor_position, start_center, handle_radius);
    let on_end = point_is_on_handle(cursor_position, end_center, handle_radius);

    if on_start && on_end && selection_is_merged(selection) {
        DragState::PendingMerged {
            origin_x: cursor_position.x,
        }
    } else if on_start && on_end {
        nearest_handle(cursor_position.x, start_center.x, end_center.x)
    } else if on_start {
        DragState::Start
    } else if on_end {
        DragState::End
    } else if selection_is_merged(selection) {
        match cursor_position.x < start_center.x {
            true => DragState::Start,
            false => DragState::End,
        }
    } else {
        nearest_handle(cursor_position.x, start_center.x, end_center.x)
    }
}

fn nearest_handle(cursor_x: f32, start_x: f32, end_x: f32) -> DragState {
    if (cursor_x - start_x).abs() <= (cursor_x - end_x).abs() {
        DragState::Start
    } else {
        DragState::End
    }
}

fn locate_value<T>(
    bounds: Rectangle,
    cursor_position: Point,
    range: &RangeInclusive<T>,
    step: T,
) -> Option<T>
where
    T: RangeValue,
{
    if cursor_position.x <= bounds.x {
        Some(*range.start())
    } else if cursor_position.x >= bounds.x + bounds.width {
        Some(*range.end())
    } else {
        let start = range.start().into_f64();
        let end = range.end().into_f64();
        let step = step.into_f64();

        if start >= end || step <= 0.0 || !step.is_finite() {
            return Some(*range.start());
        }

        let percent = f64::from(cursor_position.x - bounds.x) / f64::from(bounds.width);
        let steps = (percent * (end - start) / step).round();
        let value = (steps * step + start).clamp(start, end);

        Some(T::from_f64(value))
    }
}

fn update_selection<T, Message, F>(
    slider: &mut DoubleEndedSlider<T, Message, F>,
    shell: &mut Shell<'_, Message>,
    state: &mut State,
    active_handle: ActiveHandle,
    value: Option<T>,
) where
    T: RangeValue,
    F: Fn(RangeInclusive<T>) -> Message,
{
    let Some(value) = value else {
        return;
    };

    let start = *slider.selection.start();
    let end = *slider.selection.end();
    let next_selection = match active_handle {
        ActiveHandle::Start => {
            let next_start = if value > end { end } else { value };
            next_start..=end
        }
        ActiveHandle::End => {
            let next_end = if value < start { start } else { value };
            start..=next_end
        }
    };

    if slider.selection == next_selection {
        return;
    }

    if selection_is_merged(&next_selection) {
        state.merged_handle = active_handle;
    }

    slider.selection = next_selection.clone();
    shell.publish((slider.on_change)(next_selection));
}

fn point_is_on_handle(cursor_position: Point, handle_center: Point, handle_radius: f32) -> bool {
    let x = cursor_position.x - handle_center.x;
    let y = cursor_position.y - handle_center.y;

    (x * x) + (y * y) <= handle_radius * handle_radius
}

fn selection_is_merged<T>(selection: &RangeInclusive<T>) -> bool
where
    T: RangeValue,
{
    selection.start() == selection.end()
}

fn handle_center<T>(
    bounds: Rectangle,
    value: T,
    range: &RangeInclusive<T>,
    handle_radius: f32,
) -> Point
where
    T: RangeValue,
{
    let start = range.start().into_f64();
    let end = range.end().into_f64();
    let value = value.into_f64();
    let usable_width = (bounds.width - handle_radius * 2.0).max(0.0);
    let offset = if start >= end {
        0.0
    } else {
        usable_width * ((value - start) / (end - start)) as f32
    };

    Point::new(
        bounds.x + handle_radius + offset,
        bounds.y + bounds.height / 2.0,
    )
}

fn rail_bounds(bounds: Rectangle, rail_width: f32, handle_radius: f32) -> Rectangle {
    Rectangle {
        x: bounds.x + handle_radius,
        y: bounds.y + bounds.height / 2.0 - rail_width / 2.0,
        width: (bounds.width - handle_radius * 2.0).max(0.0),
        height: rail_width,
    }
}

fn draw_handle<Renderer>(
    renderer: &mut Renderer,
    center: Point,
    handle_radius: f32,
    handle_border: Border,
    handle_color: Color,
) where
    Renderer: renderer::Renderer,
{
    renderer.fill_quad(
        renderer::Quad {
            bounds: Rectangle {
                x: center.x - handle_radius,
                y: center.y - handle_radius,
                width: handle_radius * 2.0,
                height: handle_radius * 2.0,
            },
            border: handle_border,
            ..renderer::Quad::default()
        },
        Background::Color(handle_color),
    );
}
