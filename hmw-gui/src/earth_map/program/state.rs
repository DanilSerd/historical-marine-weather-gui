use super::super::{
    point_project_from_circle_to_unit_sphere_left_handed,
    point_project_from_unit_sphere_to_spheroid_left_handed,
};
use iced::{Event as ShaderEvent, Rectangle, event::Status as ShaderEventStatus};

use super::consts::MAX_ZOOM;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RotationMouseButton {
    Left,
    Middle,
}

#[derive(Debug, Clone, Copy)]
pub struct EarthMapState {
    rotation: glam::Quat,
    base_rotation: glam::Quat,
    zoom: f32,
    radius: f32,
    cursor_point_normal: Option<glam::Vec3>,
    cursor_point_normal_when_clicked: Option<glam::Vec3>,
    rotation_mouse_button: Option<RotationMouseButton>,
    mouse_select_state: (bool, bool),
    shift_pressed: bool,
}

impl EarthMapState {
    pub fn update(
        &mut self,
        event: &ShaderEvent,
        cursor: iced::advanced::mouse::Cursor,
        bounds: Rectangle,
    ) -> ShaderEventStatus {
        self.update_radius(bounds);
        let c1s = self.update_cursor_point(event, cursor, bounds);
        let zs = self.update_zoom(event);
        let ks = self.update_keyboard_modifiers(event);
        let ms = self.update_mouse_down(event);
        let rs = self.update_rotation();
        let c2s = self.update_cursor_point(event, cursor, bounds);
        c1s.merge(c2s).merge(zs).merge(ks).merge(ms).merge(rs)
    }

    pub fn is_rotating(&self) -> bool {
        self.cursor_point_normal_when_clicked.is_some()
    }

    pub fn rotation(&self) -> glam::Quat {
        (self.rotation * self.base_rotation).normalize()
    }

    pub fn scale(&self) -> f32 {
        self.radius
    }

    pub fn select_mouse_down(&self) -> bool {
        self.mouse_select_state.0
    }

    pub fn deselect_mouse_down(&self) -> bool {
        self.mouse_select_state.1
    }

    pub fn cursor_point_on_spheroid(&self) -> Option<glam::Vec3> {
        self.cursor_point_normal.and_then(|p| {
            point_project_from_unit_sphere_to_spheroid_left_handed(p, self.rotation())
        })
    }

    fn update_radius(&mut self, bounds: Rectangle) {
        let radius = if bounds.height < bounds.width {
            bounds.height / 2.
        } else {
            bounds.width / 2.
        };
        self.radius = radius * self.zoom;
    }

    fn update_cursor_point(
        &mut self,
        event: &ShaderEvent,
        cursor: iced::advanced::mouse::Cursor,
        bounds: Rectangle,
    ) -> ShaderEventStatus {
        let mouse_event = matches!(event, iced::Event::Mouse(..));
        if !mouse_event {
            return ShaderEventStatus::Ignored;
        }
        let cursor_point = if !cursor.is_over(bounds) {
            let s = if self.cursor_point_normal.is_some() {
                self.cursor_point_normal = None;
                ShaderEventStatus::Captured
            } else {
                ShaderEventStatus::Ignored
            };
            return s;
        } else {
            cursor
                .position_from(bounds.center())
                .expect("cursor is available")
        };

        let new_cursor_point_normal = point_project_from_circle_to_unit_sphere_left_handed(
            glam::vec2(cursor_point.x, -cursor_point.y),
            self.radius,
        );

        if new_cursor_point_normal != self.cursor_point_normal {
            self.cursor_point_normal = new_cursor_point_normal;
            ShaderEventStatus::Captured
        } else {
            ShaderEventStatus::Ignored
        }
    }
    fn update_zoom(&mut self, event: &ShaderEvent) -> ShaderEventStatus {
        if self.cursor_point_normal.is_none() {
            return ShaderEventStatus::Ignored;
        }
        let zoom_fraction = match event {
            // TODO: Looks like there's a bug with this event the event is fired but sometimes
            // deltas are 0.0
            ShaderEvent::Mouse(iced::mouse::Event::WheelScrolled { delta }) => match delta {
                iced::mouse::ScrollDelta::Lines { y, .. } => 1. / 10. * y,
                iced::mouse::ScrollDelta::Pixels { y, .. } => 1. / 100. * y,
            },
            _ => return ShaderEventStatus::Ignored,
        };

        self.zoom += self.zoom * zoom_fraction;
        self.zoom = self.zoom.clamp(1., MAX_ZOOM);
        ShaderEventStatus::Captured
    }

    fn update_keyboard_modifiers(&mut self, event: &ShaderEvent) -> ShaderEventStatus {
        if self.cursor_point_normal.is_none() {
            return ShaderEventStatus::Ignored;
        }

        let shift_pressed = match event {
            ShaderEvent::Keyboard(iced::keyboard::Event::KeyPressed { modifiers, .. }) => {
                modifiers.shift()
            }
            ShaderEvent::Keyboard(iced::keyboard::Event::KeyReleased { modifiers, .. }) => {
                modifiers.shift()
            }
            ShaderEvent::Keyboard(iced::keyboard::Event::ModifiersChanged(modifiers)) => {
                modifiers.shift()
            }
            _ => return ShaderEventStatus::Ignored,
        };

        match shift_pressed == self.shift_pressed {
            true => ShaderEventStatus::Ignored,
            false => {
                self.shift_pressed = shift_pressed;
                ShaderEventStatus::Captured
            }
        }
    }

    fn update_mouse_down(&mut self, event: &ShaderEvent) -> ShaderEventStatus {
        match event {
            ShaderEvent::Mouse(event) => match event {
                iced::mouse::Event::ButtonPressed(button) => match button {
                    iced::mouse::Button::Middle => {
                        if self.cursor_point_normal.is_none() {
                            return ShaderEventStatus::Ignored;
                        }

                        self.cursor_point_normal_when_clicked = self.cursor_point_normal;
                        self.rotation_mouse_button = Some(RotationMouseButton::Middle);
                        ShaderEventStatus::Captured
                    }
                    iced::mouse::Button::Left => {
                        if self.cursor_point_normal.is_none() {
                            return ShaderEventStatus::Ignored;
                        }

                        match self.shift_pressed {
                            true => {
                                self.cursor_point_normal_when_clicked = self.cursor_point_normal;
                                self.rotation_mouse_button = Some(RotationMouseButton::Left);
                            }
                            false => {
                                self.mouse_select_state = (true, false);
                            }
                        }
                        ShaderEventStatus::Captured
                    }
                    iced::mouse::Button::Right => {
                        if self.cursor_point_normal.is_none() {
                            return ShaderEventStatus::Ignored;
                        }
                        self.mouse_select_state = (false, true);
                        ShaderEventStatus::Captured
                    }
                    _ => ShaderEventStatus::Ignored,
                },
                iced::mouse::Event::ButtonReleased(button) => match button {
                    iced::mouse::Button::Middle => {
                        if self.rotation_mouse_button == Some(RotationMouseButton::Middle) {
                            self.stop_rotating();
                            ShaderEventStatus::Captured
                        } else {
                            ShaderEventStatus::Ignored
                        }
                    }
                    iced::mouse::Button::Left => {
                        if self.rotation_mouse_button == Some(RotationMouseButton::Left) {
                            self.stop_rotating();
                            ShaderEventStatus::Captured
                        } else if self.mouse_select_state.0 {
                            self.mouse_select_state = (false, self.mouse_select_state.1);
                            ShaderEventStatus::Captured
                        } else {
                            ShaderEventStatus::Ignored
                        }
                    }
                    iced::mouse::Button::Right => {
                        if self.mouse_select_state.1 {
                            self.mouse_select_state = (self.mouse_select_state.0, false);
                            ShaderEventStatus::Captured
                        } else {
                            ShaderEventStatus::Ignored
                        }
                    }
                    _ => ShaderEventStatus::Ignored,
                },
                _ => ShaderEventStatus::Ignored,
            },
            _ => ShaderEventStatus::Ignored,
        }
    }

    fn update_rotation(&mut self) -> ShaderEventStatus {
        let click_point = match self.cursor_point_normal_when_clicked {
            Some(p) => p,
            None => return ShaderEventStatus::Ignored,
        };

        let cursor_point = match self.cursor_point_normal {
            Some(cp) => cp,
            None => {
                self.stop_rotating();
                return ShaderEventStatus::Captured;
            }
        };

        if cursor_point == click_point {
            return ShaderEventStatus::Ignored;
        }

        self.rotation = glam::Quat::from_rotation_arc(click_point, cursor_point);
        ShaderEventStatus::Captured
    }

    fn stop_rotating(&mut self) {
        self.cursor_point_normal_when_clicked = None;
        self.rotation_mouse_button = None;
        self.base_rotation = self.rotation();
        self.rotation = glam::Quat::IDENTITY;
    }
}

impl Default for EarthMapState {
    fn default() -> Self {
        Self {
            rotation: glam::Quat::IDENTITY,
            base_rotation: glam::Quat::IDENTITY,
            zoom: 1.,
            radius: 1.,
            cursor_point_normal: None,
            cursor_point_normal_when_clicked: None,
            rotation_mouse_button: None,
            mouse_select_state: (false, false),
            shift_pressed: false,
        }
    }
}
