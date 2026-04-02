use hmw_geo::LatticeEntry;
use iced::{Element, Length, Task, widget::container};

use crate::{
    collection::WeatherSummaryCollection,
    types::{WeatherSummaryHeader, WeatherSummaryId, WeatherSummaryParams},
};

use super::{
    epoch_selector::EpochSelector,
    epoch_selector::EpochSelectorMessage,
    geo_list::LatticeNodesSelectedList,
    months_selector::{MonthsSelector, MonthsSelectorMessage},
    name::{NameSelector, NameSelectorMessage},
    weather_type_selector::{WeatherTypeSelector, WeatherTypeSelectorMessage},
};

#[derive(Debug, Clone)]
pub struct NewWeatherSummaryForm {
    pub name: NameSelector,
    weather_type_selector: WeatherTypeSelector,
    geo_list: LatticeNodesSelectedList,
    months_selector: MonthsSelector,
    epoch_selector: EpochSelector,
    pub id: WeatherSummaryId,
    pub edit: bool,
}

#[derive(Debug, Clone)]
pub enum NewWeatherSummaryFormMessage {
    Name(NameSelectorMessage),
    WeatherType(WeatherTypeSelectorMessage),
    Month(MonthsSelectorMessage),
    Epoch(EpochSelectorMessage),
    None,
    Done,
}

impl NewWeatherSummaryForm {
    pub fn new(collection: &WeatherSummaryCollection) -> Self {
        Self {
            name: NameSelector::new(String::with_capacity(0)),
            weather_type_selector: WeatherTypeSelector::new(),
            geo_list: LatticeNodesSelectedList::new(),
            months_selector: MonthsSelector::new(),
            epoch_selector: EpochSelector::new(collection.stats(), None),
            id: WeatherSummaryId::random(),
            edit: false,
        }
    }

    pub fn new_copy(params: &WeatherSummaryParams, collection: &WeatherSummaryCollection) -> Self {
        let mut form = Self::new_from_params(params.clone(), collection, false);
        form.id = WeatherSummaryId::random();
        let existing_names = collection
            .iter()
            .map(|(_, summary)| summary.params.header.name.as_str());
        form.name.update(
            NameSelectorMessage {
                name: format!("{} (copy)", params.header.name),
            },
            existing_names,
        );
        form
    }

    pub fn new_edit(params: &WeatherSummaryParams, collection: &WeatherSummaryCollection) -> Self {
        Self::new_from_params(params.clone(), collection, true)
    }

    pub fn all_geo_nodes_selected(&self) -> impl Iterator<Item = &LatticeEntry> {
        self.geo_list.nodes.iter()
    }

    /// Returns whether the form currently has any selected geo nodes.
    pub fn has_geo_selection(&self) -> bool {
        !self.geo_list.is_empty()
    }

    fn new_from_params(
        params: WeatherSummaryParams,
        collection: &WeatherSummaryCollection,
        edit: bool,
    ) -> Self {
        Self {
            name: NameSelector::new(params.header.name),
            weather_type_selector: WeatherTypeSelector {
                weather_type: params.header.summary_type,
            },
            geo_list: LatticeNodesSelectedList::new_with(params.geo),
            months_selector: MonthsSelector {
                months: params.months,
            },
            epoch_selector: EpochSelector::new(collection.stats(), Some(params.epoch)),
            id: params.header.id,
            edit,
        }
    }

    pub fn toggle_geo_selection(&mut self, node: LatticeEntry, selected: bool) {
        self.geo_list.toggle_selection(node, selected);
    }

    /// Clears all selected geo nodes from the form.
    pub fn clear_geo_selection(&mut self) {
        self.geo_list.clear();
    }

    pub fn update(
        &mut self,
        message: NewWeatherSummaryFormMessage,
        collection: &WeatherSummaryCollection,
    ) -> Task<FormSubmitted> {
        match message {
            NewWeatherSummaryFormMessage::Name(message) => {
                let existing_names = collection
                    .iter()
                    .filter(|(id, _)| &self.id != *id)
                    .map(|(_, summary)| summary.params.header.name.as_str());
                self.name.update(message, existing_names);
                Task::none()
            }
            NewWeatherSummaryFormMessage::WeatherType(message) => {
                self.weather_type_selector.weather_type = message.weather_type;
                Task::none()
            }
            NewWeatherSummaryFormMessage::Month(message) => {
                self.months_selector.toggle_month(message.month);
                Task::none()
            }
            NewWeatherSummaryFormMessage::Epoch(message) => {
                self.epoch_selector.update(message);
                Task::none()
            }
            NewWeatherSummaryFormMessage::Done => {
                let mut form = Self::new(collection);
                form.weather_type_selector.weather_type = self.weather_type_selector.weather_type;
                std::mem::swap(self, &mut form);
                Task::done(FormSubmitted {
                    params: WeatherSummaryParams {
                        header: WeatherSummaryHeader::new(
                            form.id,
                            form.name.name,
                            form.weather_type_selector.weather_type,
                        ),
                        geo: form.geo_list.nodes.into_iter().collect(),
                        months: form.months_selector.months,
                        epoch: form.epoch_selector.epoch.expect("epoch is set"),
                    },
                })
            }
            NewWeatherSummaryFormMessage::None => panic!("None message is never sent"),
        }
    }

    pub fn view(&self) -> Element<'_, NewWeatherSummaryFormMessage> {
        let ready = self.name.is_ready()
            && !self.geo_list.is_empty()
            && !self.months_selector.is_empty()
            && !self.epoch_selector.is_empty();

        let button_text = match self.edit {
            true => "Update",
            false => "Create",
        };

        let button: Element<'_, NewWeatherSummaryFormMessage> = if ready {
            iced::widget::button(button_text)
                .on_press(NewWeatherSummaryFormMessage::Done)
                .into()
        } else {
            iced::widget::button(button_text).into()
        };

        let weather_and_name_row = iced::widget::row([
            self.weather_type_selector
                .view()
                .map(NewWeatherSummaryFormMessage::WeatherType),
            self.name.view().map(NewWeatherSummaryFormMessage::Name),
        ]);

        let month_and_geo_row = iced::widget::row([
            container(
                self.months_selector
                    .view()
                    .map(NewWeatherSummaryFormMessage::Month),
            )
            .width(Length::FillPortion(1))
            .height(Length::Shrink)
            .into(),
            container(
                self.geo_list
                    .view()
                    .map(|_| NewWeatherSummaryFormMessage::None),
            )
            .width(Length::FillPortion(1))
            .height(Length::Shrink)
            .into(),
        ])
        .spacing(5)
        .width(Length::Fill)
        .height(Length::FillPortion(1));

        let epoch_row = iced::widget::row([self
            .epoch_selector
            .view()
            .map(NewWeatherSummaryFormMessage::Epoch)]);

        let button_row = iced::widget::row([button]).width(Length::Fill);

        iced::widget::column(vec![
            weather_and_name_row.into(),
            epoch_row.into(),
            month_and_geo_row.into(),
            button_row.into(),
        ])
        .spacing(5)
        .padding(5)
        .into()
    }
}

#[derive(Debug, Clone)]
pub struct FormSubmitted {
    pub params: WeatherSummaryParams,
}
