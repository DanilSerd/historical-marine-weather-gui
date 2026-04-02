use hmw_geo::LatticeEntry;
use iced::{
    Element, Font, Length,
    font::Weight,
    widget::{column, scrollable, text},
};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct LatticeNodesSelectedList {
    pub nodes: HashSet<LatticeEntry>,
}

impl LatticeNodesSelectedList {
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
        }
    }

    pub fn new_with(nodes: HashSet<LatticeEntry>) -> Self {
        Self { nodes }
    }

    pub fn toggle_selection(&mut self, node: LatticeEntry, selected: bool) {
        if selected {
            self.nodes.insert(node);
        } else {
            self.nodes.remove(&node);
        }
    }

    /// Clears all selected lattice nodes.
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    pub fn view(&self) -> Element<'_, ()> {
        let scrollable = scrollable(column(self.nodes.iter().map(|node| {
            text(node.as_ref() as &str)
                .size(10)
                .font(Font {
                    weight: Weight::Light,
                    ..Font::DEFAULT
                })
                .into()
        })))
        .width(Length::Fill)
        .height(Length::Shrink);

        if self.nodes.is_empty() {
            text("Select cells on globe.").into()
        } else {
            scrollable.into()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}
