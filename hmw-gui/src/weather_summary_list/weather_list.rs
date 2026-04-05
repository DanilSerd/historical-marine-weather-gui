use std::collections::HashMap;

use iced::{
    Element, Font, Length,
    widget::{button, container, mouse_area},
};

use crate::{
    collection::WeatherSummaryCollection,
    consts::ICON_FONT,
    types::{WeatherSummaryId, WeatherSummaryType},
    utils::icon_widget,
    widgets::follow_tooltip,
};

use super::types::{
    ALL_LIST_ITEM_CONTROL_OPTIONS, WeatherListItem, WeatherListItemControlOption,
    WeatherListItemStatus, WeatherListMessage,
};

pub struct WeatherList {
    items: HashMap<WeatherSummaryType, HashMap<WeatherSummaryId, WeatherListItem>>,
    pub type_selected: Option<WeatherSummaryType>,
    pub new_button_enabled: bool,
}

impl WeatherList {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            new_button_enabled: true,
            type_selected: None,
        }
    }

    pub fn update(&mut self, message: WeatherListMessage, collection: &WeatherSummaryCollection) {
        match message {
            WeatherListMessage::Delete(id) => {
                if let Some(summary) = collection.get(&id)
                    && let Some(map) = self.items.get_mut(&summary.params.header.summary_type)
                {
                    map.remove(&id);
                }
            }
            WeatherListMessage::Checked(summary_type, ids, selected) => {
                if self.type_selected.is_none() || self.type_selected != Some(summary_type) {
                    self.type_selected = Some(summary_type);
                }
                if let Some(map) = self.items.get_mut(&summary_type) {
                    match ids {
                        Some(id) => {
                            if let Some(i) = map.get_mut(&id) {
                                i.selected = selected;
                            }
                        }
                        None => map.values_mut().for_each(|i| i.selected = selected),
                    }
                }
                if self.all_selected().count() == 0 {
                    self.type_selected = None;
                }
            }
            WeatherListMessage::Hovered(id, hovered) => {
                if let Some(summary) = collection.get(&id)
                    && let Some(map) = self.items.get_mut(&summary.params.header.summary_type)
                    && let Some(i) = map.get_mut(&id)
                {
                    i.hovered = hovered;
                }
            }
            _ => (),
        };
    }

    pub fn add(&mut self, id: WeatherSummaryId, collection: &WeatherSummaryCollection) {
        if let Some(summary) = collection.get(&id) {
            self.items
                .entry(summary.params.header.summary_type)
                .or_default()
                .insert(id, WeatherListItem::new());
        }
    }

    pub fn view<'s>(
        &'s self,
        collection: &'s WeatherSummaryCollection,
    ) -> Element<'s, WeatherListMessage> {
        if collection.is_empty() {
            let mut b = button("Create new summary");
            if self.new_button_enabled {
                b = button("Create new summary").on_press(WeatherListMessage::New);
            }
            return container(b).center(Length::Fill).into();
        }

        let mut sorted: Vec<_> = self
            .items
            .iter()
            .filter_map(|(_wt, map)| {
                let mut items: Vec<_> = map
                    .iter()
                    .filter_map(|(id, item)| collection.get(id).map(|summary| (item, summary)))
                    .collect();
                items.sort_by_key(|(_, s)| s.params.header.id);
                if items.is_empty() {
                    return None;
                }
                Some(items)
            })
            .collect();
        sorted.sort_by_key(|v| v[0].1.params.header.summary_type);
        let iter = sorted.into_iter();

        let element = iced::widget::scrollable(iced::widget::column(iter.map(|v| {
            let element_type = v[0].1.params.header.summary_type;
            let type_is_same_as_current_selected =
                self.type_selected.is_none() || self.type_selected == Some(element_type);

            let mut all_ids_selected = true;
            let mut all_ids_available_to_select = type_is_same_as_current_selected;

            let type_list_column = iced::widget::column(v.into_iter().map(|(item, summary)| {
                if !item.selected {
                    all_ids_selected = false;
                }
                let status: WeatherListItemStatus = (&summary.data).into();
                let summary_status = match &status {
                    WeatherListItemStatus::Loading => {
                        follow_tooltip(icon_widget("⏳"), iced::widget::text("Loading"))
                    }
                    WeatherListItemStatus::Loaded => {
                        follow_tooltip(icon_widget("✔︎"), iced::widget::text("Loaded"))
                    }
                    WeatherListItemStatus::Error(e) => follow_tooltip(
                        icon_widget("❌"),
                        iced::widget::text(format!("Error: {}", e)),
                    ),
                };

                let checkbox: Element<'_, WeatherListMessage> = if status
                    == WeatherListItemStatus::Loaded
                    && type_is_same_as_current_selected
                {
                    iced::widget::checkbox(item.selected)
                        .on_toggle(move |checked| {
                            WeatherListMessage::Checked(
                                element_type,
                                Some(summary.params.header.id),
                                checked,
                            )
                        })
                        .into()
                } else {
                    all_ids_available_to_select = false;
                    iced::widget::checkbox(false).into()
                };

                let control_menu: Element<'_, _> = follow_tooltip(
                    iced::widget::pick_list(
                        &ALL_LIST_ITEM_CONTROL_OPTIONS[..],
                        Option::<WeatherListItemControlOption>::None,
                        |option| option.to_message(summary.params.header.id),
                    )
                    .style(|theme: &iced::Theme, status| {
                        let palate = theme.extended_palette();
                        let mut default = iced::widget::pick_list::default(theme, status);
                        default.background = palate.background.base.color.into();
                        default
                    })
                    .font(ICON_FONT)
                    .width(Length::Fixed(50.))
                    .placeholder("⚙︎")
                    .handle(iced::widget::pick_list::Handle::None),
                    iced::widget::text("Options"),
                );

                let mut controls = iced::widget::row([]);

                controls = controls.push(summary_status);
                controls = controls.push(control_menu);

                let font = if item.hovered {
                    Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    }
                } else {
                    Font {
                        weight: iced::font::Weight::Normal,
                        ..Default::default()
                    }
                };

                let mut name: iced::widget::MouseArea<'_, WeatherListMessage, _, _> = mouse_area(
                    iced::widget::text(&summary.params.header.name)
                        .font(font)
                        .wrapping(iced::widget::text::Wrapping::Glyph),
                )
                .on_enter(WeatherListMessage::Hovered(summary.params.header.id, true))
                .on_exit(WeatherListMessage::Hovered(summary.params.header.id, false));

                if status == WeatherListItemStatus::Loaded && type_is_same_as_current_selected {
                    name = name.on_press(WeatherListMessage::Checked(
                        element_type,
                        Some(summary.params.header.id),
                        !item.selected,
                    ));
                }
                let name: Element<'_, WeatherListMessage> = name.into();

                let row = iced::widget::row([
                    iced::widget::Space::new()
                        .width(Length::Fixed(15.0))
                        .height(Length::Fill)
                        .into(),
                    checkbox,
                    iced::widget::text(summary.params.header.summary_type.symbol())
                        .font(ICON_FONT)
                        .into(),
                    container(name).width(Length::FillPortion(3)).into(),
                    container(controls)
                        .width(Length::FillPortion(1))
                        .align_right(Length::Fill)
                        .into(),
                ])
                .spacing(6);
                row.into()
            }));

            let mut type_header_checkbox = iced::widget::checkbox(all_ids_selected);

            if all_ids_available_to_select {
                type_header_checkbox = type_header_checkbox.on_toggle(move |checked| {
                    WeatherListMessage::Checked(element_type, None, checked)
                });
            }

            let type_header = iced::widget::row([
                type_header_checkbox.into(),
                iced::widget::text(element_type.symbol())
                    .font(ICON_FONT)
                    .into(),
                iced::widget::text(element_type.to_string()).into(),
            ])
            .spacing(6);

            iced::widget::column([type_header.into(), type_list_column.into()]).into()
        })));
        element.into()
    }

    pub fn all_selected(&self) -> impl Iterator<Item = &WeatherSummaryId> {
        self.items.values().flat_map(|map| {
            map.iter()
                .filter_map(|(id, item)| item.selected.then_some(id))
        })
    }
}
