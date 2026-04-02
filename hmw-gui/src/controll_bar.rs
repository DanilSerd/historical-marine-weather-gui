use iced::{
    Element, Length,
    widget::{column, container, row, rule},
};

use crate::{
    collection::WeatherSummaryCollection,
    utils::{ControlBarIcon, control_bar_icon, tooltip_button},
    weather_summary_collection::messages::ControlBarMessage as CollectionControlBarMessage,
};

#[derive(Debug, Clone, Default)]
pub struct ControlBar;

#[derive(Debug, Clone, Copy)]
pub enum ControlBarMessage {
    NewCollection,
    OpenCollection,
    SaveCollection,
    SaveCollectionAs,
    CollectionControlBarMessage(CollectionControlBarMessage),
    OpenDataFileManager,
}

impl ControlBar {
    pub fn view<'s>(
        &'s self,
        collection_management_enabled: bool,
        collection: Option<&'s WeatherSummaryCollection>,
        collection_control_bar_extention: Option<Element<'s, CollectionControlBarMessage>>,
    ) -> Element<'s, ControlBarMessage> {
        let mut main_controls = row([
            tooltip_button(
                control_bar_icon(ControlBarIcon::Data),
                "Data",
                "Open Data File Manager",
                Some(ControlBarMessage::OpenDataFileManager),
            ),
            tooltip_button(
                control_bar_icon(ControlBarIcon::NewCollection),
                "New",
                "New Collection",
                collection_management_enabled.then_some(ControlBarMessage::NewCollection),
            ),
            tooltip_button(
                control_bar_icon(ControlBarIcon::OpenCollection),
                "Open",
                "Open Collection",
                collection_management_enabled.then_some(ControlBarMessage::OpenCollection),
            ),
        ])
        .spacing(5);

        if let Some(collection) = collection {
            let (savable, savable_as) = collection.savable();
            main_controls = main_controls.push(tooltip_button(
                control_bar_icon(ControlBarIcon::SaveCollection),
                "Save",
                "Save Collection",
                savable.then_some(ControlBarMessage::SaveCollection),
            ));
            main_controls = main_controls.push(tooltip_button(
                control_bar_icon(ControlBarIcon::SaveCollectionAs),
                "Save As",
                "Save Collection As",
                savable_as.then_some(ControlBarMessage::SaveCollectionAs),
            ));
        }

        let bar = match collection_control_bar_extention {
            Some(extention) => column([
                main_controls.into(),
                rule::horizontal(1).style(rule::default).into(),
                extention.map(ControlBarMessage::CollectionControlBarMessage),
            ])
            .spacing(5),
            None => column([main_controls.into()]).spacing(5),
        };

        container(bar)
            .padding(5)
            .width(Length::Fill)
            .height(Length::Shrink)
            .style(container::bordered_box)
            .into()
    }
}
