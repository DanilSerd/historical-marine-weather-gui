use hmw_geo::{Lattice, LatticeEntry, geo::Point};

#[derive(Debug, Clone, Copy)]
pub struct EarthMapColors {
    pub lattice_highlight: glam::Vec4,
}

impl Default for EarthMapColors {
    fn default() -> Self {
        Self {
            lattice_highlight: glam::Vec4::new(0.5, 0., 0., 0.6),
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
