use std::sync::Arc;

use hmw_geo::{Lattice, LatticeEntry};
use iced::{Rectangle, event::Status as ShaderEventStatus, widget::shader};

use crate::earth_map::left_handed_unit_spheroid_point_to_geo;
use crate::earth_map::pipelines::spheroid::Spheroid;

use super::EarthMapProgramMessage;
use super::primitive::EarthMapPrimitive;
use super::state::EarthMapState;
use super::types::{CellSelection, EarthMapColors};

#[derive(Clone, Debug)]
pub struct EarthMapProgram {
    spheroid: Arc<Spheroid>,
    lattice: Arc<Lattice>,
    cell_selection: Arc<CellSelection>,
    colors: EarthMapColors,
}

impl EarthMapProgram {
    pub fn new(spheroid: Arc<Spheroid>, colors: EarthMapColors, lattice: Arc<Lattice>) -> Self {
        Self {
            spheroid,
            colors,
            lattice,
            cell_selection: Arc::new(CellSelection::default()),
        }
    }

    pub fn set_cell_selection(&mut self, cell_selection: CellSelection) {
        self.cell_selection = Arc::new(cell_selection);
    }

    fn lattice_containing(&self, from_spheroid_point: glam::Vec3) -> Option<(LatticeEntry, usize)> {
        let p = left_handed_unit_spheroid_point_to_geo(from_spheroid_point);
        self.lattice.containing(p).map(|(e, i)| (*e, *i))
    }
}

impl iced::widget::shader::Program<EarthMapProgramMessage> for EarthMapProgram {
    type State = EarthMapState;

    type Primitive = EarthMapPrimitive;

    fn draw(
        &self,
        state: &Self::State,
        _cursor: iced::advanced::mouse::Cursor,
        _bounds: iced::Rectangle,
    ) -> Self::Primitive {
        EarthMapPrimitive {
            spheroid: self.spheroid.clone(),
            rotation: state.rotation(),
            scale: state.scale(),
            colors: self.colors,
            lattice: self.lattice.clone(),
            cell_selection: self.cell_selection.clone(),
        }
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: iced::Rectangle,
        cursor: iced::advanced::mouse::Cursor,
    ) -> Option<shader::Action<EarthMapProgramMessage>> {
        let s = state.update(event, cursor, bounds);
        if s == ShaderEventStatus::Ignored {
            return None;
        }

        let cell_message = if state.select_mouse_down() {
            let pressed_cell = state
                .cursor_point_on_spheroid()
                .and_then(|p| self.lattice_containing(p));
            pressed_cell
                .filter(|(_, i)| !self.cell_selection.selected_cells.contains(i))
                .map(|(n, _)| EarthMapProgramMessage::SelectLatticeNode(n))
        } else if state.deselect_mouse_down() {
            let pressed_cell = state
                .cursor_point_on_spheroid()
                .and_then(|p| self.lattice_containing(p));
            pressed_cell
                .filter(|(_, i)| self.cell_selection.selected_cells.contains(i))
                .map(|(n, _)| EarthMapProgramMessage::DeSelectLatticeNode(n))
        } else {
            None
        };

        let location = state
            .cursor_point_on_spheroid()
            .map(left_handed_unit_spheroid_point_to_geo);

        let m = cell_message.unwrap_or(EarthMapProgramMessage::HoverGeo(location));

        Some(shader::Action::publish(m).and_capture())
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: Rectangle,
        _cursor: iced::advanced::mouse::Cursor,
    ) -> iced::advanced::mouse::Interaction {
        if state.is_rotating() {
            iced::advanced::mouse::Interaction::Move
        } else {
            iced::advanced::mouse::Interaction::default()
        }
    }
}
