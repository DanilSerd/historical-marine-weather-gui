use hmw_geo::LatticeEntry;
use iced::{
    Element,
    widget::{row, text},
};
use std::collections::HashSet;

use crate::utils::icon_widget;

#[derive(Debug, Clone)]
pub struct LatticeNodesSelectedList {
    pub nodes: HashSet<LatticeEntry>,
    area: f64,
}

impl LatticeNodesSelectedList {
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            area: 0.,
        }
    }

    pub fn new_with(nodes: HashSet<LatticeEntry>) -> Self {
        let area = nodes.iter().map(|e| e.geodesic_area_unsigned()).sum();
        Self { nodes, area }
    }

    pub fn toggle_selection(&mut self, node: LatticeEntry, selected: bool) {
        if selected {
            let inserted = self.nodes.insert(node);
            if inserted {
                self.area += node.geodesic_area_unsigned();
            }
        } else {
            let removed = self.nodes.remove(&node);
            if removed {
                self.area -= node.geodesic_area_unsigned();
            }
        }
    }

    /// Clears all selected lattice nodes.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.area = 0.;
    }

    pub fn view(&self) -> Element<'_, ()> {
        let text = if self.nodes.is_empty() {
            text("Select cells on globe").into()
        } else {
            let area_km2 = (self.area / 1e6).round() as usize;
            let mut formatter = human_format::Formatter::new();
            formatter.with_units(" km²");
            let c = if self.nodes.len() > 1 {
                "cells"
            } else {
                "cell"
            };
            text(format!(
                "{} {} - {}",
                self.nodes.len(),
                c,
                formatter.format(area_km2 as f64),
            ))
            .into()
        };
        let icon = icon_widget("🌍");
        row([icon.into(), text]).spacing(5).into()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}
