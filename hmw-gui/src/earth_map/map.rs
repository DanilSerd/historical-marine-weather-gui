use std::{borrow::Cow, sync::Arc};

use hmw_geo::{Lattice, LatticeEntry, geo::Point};
use iced::{
    Alignment, Element, Length, alignment,
    widget::{button, column, container, mouse_area, row, scrollable, stack, svg, text},
};

use super::{
    Spheroid,
    pipelines::spheroid::SpheroidTextureData,
    program::{CellSelection, EarthMapColors, EarthMapProgram, EarthMapProgramMessage},
};
use crate::{assets::Assets, widgets::follow_tooltip};

const DEFAULT_SUBDIVISIONS: usize = 20;
const HELP_ICON_SIZE: f32 = 64.0;

#[derive(Debug, Clone)]
pub struct EarthMap {
    program: EarthMapProgram,
    lattice: Arc<Lattice>,
    hover_geo: Option<Point>,
    show_help: bool,
}

#[derive(Debug, Clone)]
pub enum EarthMapMessage {
    /// A map cell should be toggled in the current selection.
    SelectLatticeNode(LatticeEntry, bool),
    /// The current form selection should be cleared.
    ClearSelection,
    /// The help overlay visibility changed.
    ToggleHelp(bool),
    /// The cursor moved to a new globe location.
    HoverGeo(Option<Point>),
    /// An overlay interaction was intentionally swallowed.
    Ignored,
}

impl EarthMap {
    /// Creates a new interactive earth map.
    pub fn new<'a>(
        colors: EarthMapColors,
        lattice: Arc<Lattice>,
        texture_bytes: Cow<'a, [u8]>,
    ) -> Result<Self, &'static str> {
        let texture = SpheroidTextureData::load(&texture_bytes)?;
        let spheroid = Spheroid::new(DEFAULT_SUBDIVISIONS, texture);
        Ok(Self {
            program: EarthMapProgram::new(Arc::new(spheroid), colors, lattice.clone()),
            lattice,
            hover_geo: None,
            show_help: false,
        })
    }

    /// Updates the local UI state of the map.
    pub fn update(&mut self, message: EarthMapMessage) {
        match message {
            EarthMapMessage::ToggleHelp(show_help) => {
                self.show_help = show_help;
            }
            EarthMapMessage::HoverGeo(hover_geo) => {
                self.hover_geo = hover_geo;
            }
            _ => {}
        }
    }

    /// Sets highlighted and selected lattice nodes.
    pub fn set_highlight_and_select<'a>(
        &'a mut self,
        highlight_cells: impl IntoIterator<Item = &'a LatticeEntry>,
        select_cells: impl IntoIterator<Item = &'a LatticeEntry>,
    ) {
        self.program.set_cell_selection(CellSelection::new(
            &self.lattice,
            select_cells,
            highlight_cells,
        ));
    }

    /// Clears the current map selection state.
    pub fn clear(&mut self) {
        self.program.set_cell_selection(CellSelection::default());
        self.hover_geo = None;
        self.show_help = false;
    }

    /// Builds the earth map view.
    pub fn view(&self, show_clear_selection: bool) -> Element<'_, EarthMapMessage> {
        let map: Element<'_, EarthMapProgramMessage> = iced::widget::shader(&self.program)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        let map: Element<'_, EarthMapMessage> = map.map(|m| match m {
            EarthMapProgramMessage::SelectLatticeNode(n) => {
                EarthMapMessage::SelectLatticeNode(n, true)
            }
            EarthMapProgramMessage::DeSelectLatticeNode(n) => {
                EarthMapMessage::SelectLatticeNode(n, false)
            }
            EarthMapProgramMessage::HoverGeo(point) => EarthMapMessage::HoverGeo(point),
        });

        let mut buttons: Vec<Element<'_, EarthMapMessage>> = vec![];

        if show_clear_selection {
            buttons.push(follow_tooltip(
                button(text("Clear").size(16))
                    .padding([6, 12])
                    .style(button::danger)
                    .on_press(EarthMapMessage::ClearSelection),
                text("Clear selected cells"),
            ));
        }

        buttons.push(
            button(text("?").size(24))
                .padding([5, 12])
                .style(button::secondary)
                .on_press(EarthMapMessage::ToggleHelp(true))
                .into(),
        );

        let action_buttons: Element<'_, EarthMapMessage> =
            container(column(buttons).spacing(8).align_x(Alignment::Start))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(alignment::Horizontal::Left)
                .align_y(alignment::Vertical::Bottom)
                .padding(16)
                .into();

        let help_overlay = if self.show_help {
            self.help_overlay_view()
        } else {
            container(text(""))
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into()
        };

        let hover_coordinates = self.hover_coordinates_view();

        stack([map, action_buttons, hover_coordinates, help_overlay]).into()
    }

    fn hover_coordinates_view(&self) -> Element<'_, EarthMapMessage> {
        let Some(point) = self.hover_geo else {
            return container(text(""))
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into();
        };

        let label = format!("Lat: {:.2}, Lon: {:.2}", point.y(), point.x());

        container(
            container(text(label).size(15))
                .padding([6, 10])
                .style(style::hover_coordinates),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Right)
        .align_y(alignment::Vertical::Bottom)
        .padding(16)
        .into()
    }

    fn help_overlay_view(&self) -> Element<'_, EarthMapMessage> {
        let header = row([
            text("Globe Controls").size(24).into(),
            container(
                button(text("Close"))
                    .style(button::secondary)
                    .on_press(EarthMapMessage::ToggleHelp(false)),
            )
            .width(Length::Fill)
            .align_x(alignment::Horizontal::Right)
            .into(),
        ])
        .align_y(Alignment::Center)
        .spacing(12);

        let help_items = column([
            help_row(
                Assets::earth_map_help_select_svg(),
                "Select region",
                "Left click selects the region under the cursor.",
            ),
            help_row(
                Assets::earth_map_help_deselect_svg(),
                "De-select region",
                "Right click removes the region under the cursor from the selection.",
            ),
            help_row(
                Assets::earth_map_help_zoom_svg(),
                "Zoom",
                "Scroll to zoom the globe in and out.",
            ),
            help_row(
                Assets::earth_map_help_rotate_svg(),
                "Rotate",
                "Hold Shift and drag with the left mouse button, or drag with the middle mouse button, to rotate the globe.",
            ),
        ])
        .spacing(16)
        .width(Length::Fill);

        let help_body = column([
            header.into(),
            scrollable(help_items).width(Length::Fill).into(),
        ])
        .spacing(16)
        .width(Length::Fill)
        .height(Length::Fill);

        let card: Element<'_, EarthMapMessage> = container(help_body)
            .width(Length::Fill)
            .max_width(560)
            .height(Length::Fill)
            .padding(20)
            .style(container::bordered_box)
            .into();

        let backdrop: Element<'_, EarthMapMessage> = mouse_area(
            container(text(""))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(style::help_backdrop),
        )
        .on_press(EarthMapMessage::Ignored)
        .on_release(EarthMapMessage::Ignored)
        .on_right_press(EarthMapMessage::Ignored)
        .on_right_release(EarthMapMessage::Ignored)
        .on_middle_press(EarthMapMessage::Ignored)
        .on_middle_release(EarthMapMessage::Ignored)
        .on_scroll(|_| EarthMapMessage::Ignored)
        .on_move(|_| EarthMapMessage::Ignored)
        .into();

        stack([
            backdrop,
            container(card)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(24)
                .max_height(450)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
        ])
        .into()
    }
}

fn help_row(
    svg_bytes: Cow<'static, [u8]>,
    title: &'static str,
    blurb: &'static str,
) -> Element<'static, EarthMapMessage> {
    let icon = svg(iced::widget::svg::Handle::from_memory(svg_bytes))
        .width(Length::Fixed(HELP_ICON_SIZE))
        .height(Length::Fixed(HELP_ICON_SIZE));

    row([
        container(icon).width(Length::Fixed(HELP_ICON_SIZE)).into(),
        column([text(title).size(18).into(), text(blurb).size(14).into()])
            .spacing(4)
            .width(Length::Fill)
            .into(),
    ])
    .spacing(16)
    .align_y(Alignment::Center)
    .into()
}

mod style {
    use iced::{Theme, widget::container};

    pub fn help_backdrop(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(palette.secondary.base.color.scale_alpha(0.6).into()),
            ..container::Style::default()
        }
    }

    pub fn hover_coordinates(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            text_color: Some(palette.background.base.text),
            background: Some(palette.secondary.base.color.scale_alpha(0.6).into()),
            ..container::Style::default()
        }
    }
}
