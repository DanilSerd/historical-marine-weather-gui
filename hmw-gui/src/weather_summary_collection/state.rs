use hmw_data::BeaufortScaleBucketer;
use iced::{
    Element, Task,
    widget::{pane_grid, row},
};

use crate::{
    collection::WeatherSummaryCollection,
    earth_map::{EarthMap, EarthMapMessage},
    types::{WeatherSummaryData, WeatherSummaryType},
    utils::{ControlBarIcon, control_bar_icon, tooltip_button},
    weather_summary_details::WeatherSummaryDetails,
    weather_summary_form::NewWeatherSummaryForm,
    weather_summary_list::{WeatherList, WeatherListMessage},
    windrose::{WindRose, WindRoseColorStrategy},
};

use super::{
    WeatherSummaryCollectionMessage,
    messages::{ControlBarMessage, PaneMessage},
    screen::{MainScreenPaneState, Pane, Screen, ScreenSelection, WindrosePaneState, content},
};

pub struct WeatherSummaryCollectionScreensState {
    pub collection: WeatherSummaryCollection,
    new_summary_form: Option<NewWeatherSummaryForm>,
    earth_map: EarthMap,
    wind_display: WindRose<BeaufortScaleBucketer>,
    summary_details: WeatherSummaryDetails,
    summary_list: WeatherList,
    screen: Screen,
}

impl WeatherSummaryCollectionScreensState {
    pub fn new(
        collection: WeatherSummaryCollection,
        earth_map: EarthMap,
        wind_rose_color_strategy: WindRoseColorStrategy,
    ) -> Self {
        let mut s = Self {
            collection,
            new_summary_form: None,
            earth_map,
            wind_display: WindRose::new(wind_rose_color_strategy.clone()),
            summary_details: WeatherSummaryDetails::default(),
            summary_list: WeatherList::new(),
            screen: Screen::default(),
        };
        s.collection.iter().for_each(|(id, summary)| {
            s.summary_list.add(*id, &s.collection);
            s.summary_details.add(*id);
            match &summary.data {
                WeatherSummaryData::Wind(h) => {
                    s.wind_display.insert(id, h, false);
                }
                WeatherSummaryData::Error(_) => (),
                WeatherSummaryData::None => (),
            }
        });
        s
    }

    pub fn close(self) -> (EarthMap, WeatherSummaryCollection) {
        (self.earth_map, self.collection)
    }

    pub fn update(
        &mut self,
        message: WeatherSummaryCollectionMessage,
    ) -> Task<WeatherSummaryCollectionMessage> {
        let mut map_selection_changed = false;
        let mut map_highlight_changed = false;
        let mut summary_selection_changed = false;
        let task = match message {
            WeatherSummaryCollectionMessage::EarthMapMessage(m) => {
                match m {
                    EarthMapMessage::SelectLatticeNode(n, select) => {
                        if let Some(f) = self.new_summary_form.as_mut() {
                            f.toggle_geo_selection(n, select);
                        }
                        map_selection_changed = true;
                    }
                    EarthMapMessage::ClearSelection => {
                        if let Some(f) = self.new_summary_form.as_mut() {
                            f.clear_geo_selection();
                            map_selection_changed = true;
                        }
                    }
                    m @ (EarthMapMessage::ToggleHelp(_)
                    | EarthMapMessage::HoverGeo(_)
                    | EarthMapMessage::Ignored) => {
                        self.earth_map.update(m);
                    }
                };
                Task::none()
            }
            WeatherSummaryCollectionMessage::WeatherListMessage(m) => {
                match &m {
                    WeatherListMessage::Delete(id) => {
                        self.collection.remove(id);
                        self.wind_display.remove(id);
                        summary_selection_changed = true;
                        if self
                            .new_summary_form
                            .as_ref()
                            .map(|f| f.id == *id)
                            .unwrap_or(false)
                            && let Some(f) = self.new_summary_form.as_mut()
                        {
                            f.edit = false;
                        }
                    }
                    WeatherListMessage::Checked(_, _, _) => {
                        summary_selection_changed = true;
                    }
                    WeatherListMessage::Hovered(_, _) => (),
                    WeatherListMessage::Edit(id) => {
                        self.new_summary_form = self
                            .collection
                            .get(id)
                            .map(|s| NewWeatherSummaryForm::new_edit(&s.params, &self.collection));
                        self.screen.open_form();
                        map_selection_changed = true;
                    }
                    WeatherListMessage::Duplicate(id) => {
                        self.new_summary_form = self
                            .collection
                            .get(id)
                            .map(|s| NewWeatherSummaryForm::new_copy(&s.params, &self.collection));
                        self.screen.open_form();
                        map_selection_changed = true;
                    }
                    WeatherListMessage::New => {
                        self.new_summary_form = Some(NewWeatherSummaryForm::new(&self.collection));
                        self.screen.open_form();
                        map_selection_changed = true;
                    }
                };
                self.summary_list.update(m, &self.collection);
                Task::none()
            }
            WeatherSummaryCollectionMessage::WindRoseMessage(m, t) => {
                match t {
                    WeatherSummaryType::Wind => {
                        self.wind_display
                            .update(m, self.collection.iter_wind_data());
                    }
                };
                Task::none()
            }
            WeatherSummaryCollectionMessage::NewWeatherSummaryFormMessage(m) => self
                .new_summary_form
                .as_mut()
                .map(|f| {
                    f.update(m, &self.collection)
                        .map(WeatherSummaryCollectionMessage::WeatherSummaryFormSubmitted)
                })
                .unwrap_or(Task::none()),
            WeatherSummaryCollectionMessage::WeatherSummaryFormSubmitted(form_submitted) => {
                let id = form_submitted.params.header.id;
                let task = self
                    .collection
                    .add(form_submitted.params)
                    .map(|(id, data)| WeatherSummaryCollectionMessage::SummaryLoaded(id, data));
                self.summary_list.add(id, &self.collection);
                self.summary_details.add(id);
                map_selection_changed = true;
                summary_selection_changed = true;
                task
            }
            WeatherSummaryCollectionMessage::SummaryLoaded(id, data) => {
                self.collection.finish_load(&id, data);
                if let Some(s) = self.collection.get(&id) {
                    match &s.data {
                        WeatherSummaryData::Wind(h) => {
                            self.wind_display.insert(&id, h, false);
                        }
                        WeatherSummaryData::Error(_) | WeatherSummaryData::None => {}
                    }
                }
                Task::none()
            }
            WeatherSummaryCollectionMessage::ControlBarMessage(m) => match m {
                ControlBarMessage::ToggleScreen => {
                    self.screen.toggle_screen();
                    Task::none()
                }
                ControlBarMessage::NewForm => {
                    self.new_summary_form = Some(NewWeatherSummaryForm::new(&self.collection));
                    self.screen.open_form();
                    map_selection_changed = true;
                    Task::none()
                }
            },
            WeatherSummaryCollectionMessage::PaneMessage(m) => {
                match m {
                    PaneMessage::Resized(e) => {
                        self.screen.resize(&e);
                    }
                    PaneMessage::CloseForm => {
                        self.screen.close_form();
                        self.new_summary_form = None;
                        map_selection_changed = true;
                    }
                    PaneMessage::CloseWindRose => {
                        self.screen.selection = ScreenSelection::Main;
                    }
                    PaneMessage::None => (),
                }
                Task::none()
            }
            WeatherSummaryCollectionMessage::WeatherSummaryDetailsMessage(m) => {
                self.summary_details.update(m);
                map_highlight_changed = true;
                Task::none()
            }
            WeatherSummaryCollectionMessage::None => Task::none(),
        };

        if summary_selection_changed {
            self.wind_display
                .set_visible(self.summary_list.all_selected());
            self.summary_details
                .set_visible(self.summary_list.all_selected());
        }

        if map_highlight_changed || summary_selection_changed || map_selection_changed {
            let iter_highlighted = self
                .summary_details
                .all_show_points_on_map()
                .filter_map(|id| self.collection.get(id).map(|d| d.params.geo.iter()))
                .flatten();
            match &self.new_summary_form {
                Some(f) => self
                    .earth_map
                    .set_highlight_and_select(iter_highlighted, f.all_geo_nodes_selected()),
                None => self
                    .earth_map
                    .set_highlight_and_select(iter_highlighted, std::iter::empty()),
            };
        }

        self.summary_list.new_button_enabled = self.new_summary_form.is_none();

        task
    }

    pub fn view(&self) -> Element<'_, WeatherSummaryCollectionMessage> {
        match &self.screen.selection {
            ScreenSelection::Main => self.view_main(&self.screen.main),
            ScreenSelection::Windrose => self.view_windrose(&self.screen.windrose),
        }
    }

    pub fn view_control_bar_extension(&self) -> Element<'_, ControlBarMessage> {
        let summary_or_main = match self.screen.selection {
            ScreenSelection::Main => tooltip_button(
                control_bar_icon(ControlBarIcon::ViewSummaries),
                "View",
                "View Summaries",
                (self.summary_list.all_selected().count() > 0)
                    .then_some(ControlBarMessage::ToggleScreen),
            ),
            ScreenSelection::Windrose => tooltip_button(
                control_bar_icon(ControlBarIcon::ViewMain),
                "Main",
                "View Main Screen",
                Some(ControlBarMessage::ToggleScreen),
            ),
        };
        let new_form = tooltip_button(
            control_bar_icon(ControlBarIcon::NewSummary),
            "Summary",
            "New Summary",
            self.new_summary_form
                .is_none()
                .then_some(ControlBarMessage::NewForm),
        );

        row([new_form, summary_or_main]).spacing(5).into()
    }

    fn selected_summaries_type(&self) -> Option<WeatherSummaryType> {
        self.summary_list.type_selected
    }

    fn view_windrose<'a>(
        &'a self,
        pane_state: &'a WindrosePaneState,
    ) -> Element<'a, WeatherSummaryCollectionMessage> {
        pane_grid(pane_state.state(), |_, pane, _| self.view_pane(pane))
            .on_resize(10, |e| {
                WeatherSummaryCollectionMessage::PaneMessage(PaneMessage::Resized(e))
            })
            .spacing(10)
            .into()
    }

    fn view_main<'a>(
        &'a self,
        pane_state: &'a MainScreenPaneState,
    ) -> Element<'a, WeatherSummaryCollectionMessage> {
        pane_grid(pane_state.state(), |_, pane, _| self.view_pane(pane))
            .on_resize(10, |e| {
                WeatherSummaryCollectionMessage::PaneMessage(PaneMessage::Resized(e))
            })
            .spacing(10)
            .into()
    }

    fn view_pane<'a>(
        &'a self,
        pane: &Pane,
    ) -> pane_grid::Content<'a, WeatherSummaryCollectionMessage> {
        match pane {
            Pane::EarthMap => content(
                self.earth_map
                    .view(
                        self.new_summary_form
                            .as_ref()
                            .map(NewWeatherSummaryForm::has_geo_selection)
                            .unwrap_or(false),
                    )
                    .map(WeatherSummaryCollectionMessage::EarthMapMessage),
                "Globe",
                None,
            ),
            Pane::WeatherSummaryList => content(
                self.summary_list
                    .view(&self.collection)
                    .map(WeatherSummaryCollectionMessage::WeatherListMessage),
                format!("Summaries ({})", self.collection.iter().count()),
                None,
            ),
            Pane::WeatherSummaryForm => {
                let form = self
                    .new_summary_form
                    .as_ref()
                    .expect("new summary pane is open");
                let title = if form.edit {
                    format!(
                        "Edit Summary ({})",
                        self.collection
                            .get(&form.id)
                            .map(|s| s.params.header.name.as_str())
                            .unwrap_or_default()
                    )
                } else {
                    "New Summary".to_string()
                };
                content(
                    form.view()
                        .map(WeatherSummaryCollectionMessage::NewWeatherSummaryFormMessage),
                    title,
                    Some(PaneMessage::CloseForm),
                )
            }
            Pane::WindRose => {
                let (rose, title, rose_type) = match self.selected_summaries_type() {
                    Some(WeatherSummaryType::Wind) => (
                        self.wind_display.view_wind_rose(&self.collection),
                        "Wind Rose",
                        WeatherSummaryType::Wind,
                    ),
                    None => (
                        self.wind_display.view_wind_rose(&self.collection),
                        "Wind Rose",
                        WeatherSummaryType::Wind,
                    ),
                };
                content(
                    rose.map(move |m| {
                        WeatherSummaryCollectionMessage::WindRoseMessage(m, rose_type)
                    }),
                    title,
                    Some(PaneMessage::CloseWindRose),
                )
            }
            Pane::WeatherSummaryDetails => content(
                self.summary_details
                    .view(&self.collection)
                    .map(WeatherSummaryCollectionMessage::WeatherSummaryDetailsMessage),
                "Summary Details",
                None,
            ),
            Pane::WindRoseSidePanel => {
                let rose_type = self
                    .selected_summaries_type()
                    .unwrap_or(WeatherSummaryType::Wind);
                let rose = match rose_type {
                    WeatherSummaryType::Wind => self.wind_display.view_sidepanel(),
                };
                content(
                    rose.map(move |m| {
                        WeatherSummaryCollectionMessage::WindRoseMessage(m, rose_type)
                    }),
                    "Key and controls",
                    None,
                )
            }
        }
    }
}
