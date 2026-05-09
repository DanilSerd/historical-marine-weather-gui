use hmw_geo::{Lattice, LatticeEntry, geo::Point};
use iced::Theme;

#[derive(Debug, Clone, Copy)]
pub struct EarthMapColors {
    pub lattice_highlight: glam::Vec4,
    pub dark_mode: bool,
}

impl EarthMapColors {
    pub fn from_theme(theme: &Theme) -> Self {
        let palette = theme.extended_palette();
        let color = palette.primary.base.color;

        Self {
            lattice_highlight: glam::Vec4::new(color.r, color.g, color.b, 0.80),
            dark_mode: palette.is_dark,
        }
    }
}

#[derive(Debug, Clone)]
pub enum EarthMapProgramMessage {
    SelectLatticeNode(LatticeEntry),
    DeSelectLatticeNode(LatticeEntry),
    HoverGeo(Option<Point>),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CellSelection {
    pub selected_cells: Box<[usize]>,
    pub highlight_cells: Box<[usize]>,
}

impl CellSelection {
    pub fn new<'l>(
        lattice: &'l Lattice,
        selected_cells: impl IntoIterator<Item = &'l LatticeEntry>,
        hilight_cells: impl IntoIterator<Item = &'l LatticeEntry>,
    ) -> Self {
        let selected_cells = selected_cells
            .into_iter()
            .map(|n| *lattice.lookup(n).unwrap())
            .collect::<Box<_>>();
        let highlight_cells = hilight_cells
            .into_iter()
            .map(|n| *lattice.lookup(n).unwrap())
            .filter(|n| !selected_cells.contains(n))
            .collect::<Box<_>>();
        Self {
            selected_cells,
            highlight_cells,
        }
    }
}
