use std::fmt;
use std::fmt::Debug;

use iced::widget::{Space, container, row, text};
use iced::{Alignment, Element, Font, Length, Task, font::Weight};
use rfd::FileHandle;

use crate::app::persistant_state::AppPersistentConfig;
use crate::collection::{SavingStatus, WeatherSummaryCollection};
use crate::controll_bar::{ControlBar, ControlBarMessage};
use crate::earth_map::{EarthMap, EarthMapColors};
use crate::loader::Loader;
use crate::weather_summary_collection::{
    WeatherSummaryCollectionMessage, WeatherSummaryCollectionScreensState,
};

#[derive(Default)]

pub struct MainWindowState {
    collection_state: Option<WeatherSummaryCollectionScreensState>,
    earth_map: Option<EarthMap>,
    loader: Option<Loader>,
    control_bar: ControlBar,
}

pub enum MainWindowMessage {
    OpenCollection(FileHandle),
    CollectionError(String),
    CollectionSaveAs(FileHandle),
    CollectionSaved(FileHandle),
    CollectionOpened(WeatherSummaryCollection),
    NewCollection,
    CollectionMessage(WeatherSummaryCollectionMessage),
    ControlBarMessage(ControlBarMessage),
    None,
}

impl MainWindowState {
    pub fn new(loader: Option<Loader>, earth_map: EarthMap) -> Self {
        Self {
            collection_state: None,
            earth_map: Some(earth_map),
            loader,
            control_bar: ControlBar,
        }
    }

    pub fn update_loader(&mut self, loader: Loader) -> Task<MainWindowMessage> {
        self.loader = Some(loader.clone());
        if let Some(mut collection) = self.close_collection_view() {
            collection.start_refresh(loader);
            Task::done(MainWindowMessage::CollectionOpened(collection))
        } else {
            Task::none()
        }
    }

    pub fn update(&mut self, message: MainWindowMessage) -> Task<MainWindowMessage> {
        match message {
            MainWindowMessage::None => Task::none(),
            MainWindowMessage::CollectionMessage(message) => self
                .collection_state
                .as_mut()
                .map(|state| {
                    state
                        .update(message)
                        .map(MainWindowMessage::CollectionMessage)
                })
                .unwrap_or(Task::none()),
            MainWindowMessage::NewCollection => {
                self.close_collection_view();
                self.collection_state = Some(WeatherSummaryCollectionScreensState::new(
                    WeatherSummaryCollection::new(self.loader.clone().unwrap()),
                    self.earth_map.take().unwrap(),
                    Default::default(),
                ));
                Task::none()
            }
            MainWindowMessage::OpenCollection(fh) => {
                self.close_collection_view();
                WeatherSummaryCollection::open(fh, self.loader.clone().unwrap()).map(|c| match c {
                    Ok(c) => MainWindowMessage::CollectionOpened(c),
                    Err(e) => MainWindowMessage::CollectionError(e.to_string()),
                })
            }
            MainWindowMessage::CollectionOpened(c) => {
                self.collection_state = Some(WeatherSummaryCollectionScreensState::new(
                    c,
                    self.earth_map.take().unwrap(),
                    Default::default(),
                ));

                self.collection_state
                    .as_mut()
                    .unwrap()
                    .collection
                    .finish_open()
                    .map(WeatherSummaryCollectionMessage::SummaryLoaded)
                    .map(MainWindowMessage::CollectionMessage)
            }
            MainWindowMessage::CollectionError(e) => {
                let future = async move {
                    rfd::AsyncMessageDialog::new()
                        .set_level(rfd::MessageLevel::Error)
                        .set_description(e)
                        .set_buttons(rfd::MessageButtons::Ok)
                        .set_title("Collection Error!")
                        .show()
                        .await;
                };
                Task::future(future).map(|_| MainWindowMessage::None)
            }
            MainWindowMessage::CollectionSaved(fh) => {
                if let Some(s) = self.collection_state.as_mut() {
                    s.collection.finish_save(fh);
                }
                Task::none()
            }
            MainWindowMessage::CollectionSaveAs(fh) => self
                .collection_state
                .as_mut()
                .map(|s| {
                    let r = s.collection.change_file(fh);
                    let m = match r {
                        Ok(_) => {
                            MainWindowMessage::ControlBarMessage(ControlBarMessage::SaveCollection)
                        }
                        Err(e) => MainWindowMessage::CollectionError(e.to_string()),
                    };
                    Task::done(m)
                })
                .unwrap_or(Task::none()),
            MainWindowMessage::ControlBarMessage(m) => match m {
                ControlBarMessage::NewCollection => self
                    .proceede_with_close_of_collection_view()
                    .map(|proceed| match proceed {
                        true => MainWindowMessage::NewCollection,
                        false => MainWindowMessage::None,
                    }),
                ControlBarMessage::OpenCollection => self
                    .proceede_with_close_of_collection_view()
                    .then(|proceed| match proceed {
                        true => {
                            let future = async {
                                rfd::AsyncFileDialog::new()
                                    .add_filter("json", &["json"])
                                    .pick_file()
                                    .await
                            };
                            Task::future(future).map(|f| match f {
                                Some(fh) => MainWindowMessage::OpenCollection(fh),
                                None => MainWindowMessage::None,
                            })
                        }
                        false => Task::none(),
                    }),
                ControlBarMessage::SaveCollection => match &mut self.collection_state {
                    Some(state) => match state.collection.save() {
                        Ok(task) => task.map(|r| match r {
                            Ok(fh) => MainWindowMessage::CollectionSaved(fh),
                            Err(e) => MainWindowMessage::CollectionError(e.to_string()),
                        }),
                        Err(e) => Task::done(MainWindowMessage::CollectionError(e.to_string())),
                    },
                    None => Task::none(),
                },
                ControlBarMessage::SaveCollectionAs => {
                    let future = async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("json", &["json"])
                            .save_file()
                            .await
                    };
                    let (task, _handle) = Task::future(future)
                        .map(|f| match f {
                            Some(fh) => MainWindowMessage::CollectionSaveAs(fh),
                            None => MainWindowMessage::None,
                        })
                        .abortable();
                    task
                }
                ControlBarMessage::CollectionControlBarMessage(control_bar_message) => self
                    .collection_state
                    .as_mut()
                    .map(|s| {
                        s.update(WeatherSummaryCollectionMessage::ControlBarMessage(
                            control_bar_message,
                        ))
                        .map(MainWindowMessage::CollectionMessage)
                    })
                    .unwrap_or(Task::none()),
                ControlBarMessage::ToggleDarkMode(dark_mode) => {
                    let earth = self
                        .earth_map
                        .as_mut()
                        .or(self.collection_state.as_mut().map(|c| &mut c.earth_map));
                    if let Some(earth) = earth {
                        match dark_mode {
                            true => earth.set_colors(EarthMapColors::dark()),
                            false => earth.set_colors(EarthMapColors::light()),
                        }
                    }
                    Task::none()
                }
                _ => Task::none(),
            },
        }
    }

    fn proceede_with_close_of_collection_view(&self) -> Task<bool> {
        match &self.collection_state {
            Some(state) if state.collection.save_details().1 == SavingStatus::Unsaved => {
                let future = async {
                    let r = rfd::AsyncMessageDialog::new()
                        .set_level(rfd::MessageLevel::Warning)
                        .set_description("Do you want to proceed without saving?")
                        .set_buttons(rfd::MessageButtons::YesNo)
                        .set_title("Unsaved collection!")
                        .show()
                        .await;
                    matches!(
                        r,
                        rfd::MessageDialogResult::Yes | rfd::MessageDialogResult::Ok
                    )
                };
                Task::future(future)
            }
            _ => Task::done(true),
        }
    }

    fn close_collection_view(&mut self) -> Option<WeatherSummaryCollection> {
        match &mut self.collection_state {
            st @ Some(_) => {
                let state = st.take().unwrap();
                let (mut earth_map, collection) = state.close();
                earth_map.clear();
                self.earth_map = Some(earth_map);
                Some(collection)
            }
            None => None,
        }
    }

    pub fn view(&self, config: Option<&AppPersistentConfig>) -> Element<'_, MainWindowMessage> {
        let (collection_state_view, collection, collection_view_bar_extention) =
            match &self.collection_state {
                Some(collection_state) => (
                    Some(
                        collection_state
                            .view()
                            .map(MainWindowMessage::CollectionMessage),
                    ),
                    Some(&collection_state.collection),
                    Some(collection_state.view_control_bar_extension()),
                ),
                None => (None, None, None),
            };

        let body = collection_state_view
            .unwrap_or(Space::new().width(Length::Fill).height(Length::Fill).into());

        let control_bar = self
            .control_bar
            .view(
                self.loader.is_some(),
                collection,
                collection_view_bar_extention,
                config.and_then(|c| c.dark_mode).unwrap_or_default(),
            )
            .map(MainWindowMessage::ControlBarMessage);
        let main_content = iced::widget::column([
            container(control_bar).height(Length::Shrink).into(),
            container(body).height(Length::Fill).into(),
        ])
        .spacing(10);

        iced::widget::column([
            main_content.into(),
            container(self.view_footer())
                .height(Length::Shrink)
                .padding([6, 10])
                .into(),
        ])
        .into()
    }

    fn view_footer(&self) -> Element<'_, MainWindowMessage> {
        let collection = match self.collection_state.as_ref() {
            Some(c) => &c.collection,
            None => return Space::new().width(Length::Fill).height(Length::Fill).into(),
        };

        let (save_file_path, save_status) = collection.save_details();
        let mut footer_row = row([]).spacing(12).align_y(Alignment::Center);
        if let Some(path) = save_file_path {
            footer_row = footer_row.push(
                row([
                    text("File:")
                        .font(Font {
                            weight: Weight::Bold,
                            ..Font::DEFAULT
                        })
                        .into(),
                    text(path.to_string_lossy()).into(),
                ])
                .spacing(4)
                .align_y(Alignment::Center),
            );
        }
        footer_row = footer_row.push(
            text(save_status.to_string()).style(move |theme| save_status_style(theme, save_status)),
        );

        footer_row.into()
    }
}

fn save_status_style(theme: &iced::Theme, status: SavingStatus) -> iced::widget::text::Style {
    let palette = theme.extended_palette();
    let color = match status {
        SavingStatus::Saved => palette.success.base.color,
        SavingStatus::Saving => palette.primary.base.color,
        SavingStatus::Unsaved => palette.warning.base.color,
    };

    iced::widget::text::Style { color: Some(color) }
}

impl Debug for MainWindowMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MainWindowMessage::OpenCollection(_) => "OpenCollection",
            MainWindowMessage::CollectionError(_) => "CollectionError",
            MainWindowMessage::CollectionSaveAs(_) => "CollectionSaveAs",
            MainWindowMessage::CollectionSaved(_) => "CollectionSaved",
            MainWindowMessage::CollectionOpened(_) => "CollectionOpened",
            MainWindowMessage::NewCollection => "NewCollection",
            MainWindowMessage::CollectionMessage(_) => "CollectionMessage",
            MainWindowMessage::ControlBarMessage(_) => "ControlBarMessage",
            MainWindowMessage::None => "None",
        })
    }
}
