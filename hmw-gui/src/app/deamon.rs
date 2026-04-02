use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use hmw_geo::Lattice;
use iced::{
    Element, Length, Size, Subscription, Task,
    widget::{container, svg, text},
    window,
};

use crate::{
    app::{
        main_window::{MainWindowMessage, MainWindowState},
        persistant_state::AppPersistentStateManager,
    },
    assets::Assets,
    data_file_manager::{DataFileManager, Message as DataFileManagerMessage},
    earth_map::EarthMap,
    loader::Loader,
};

#[derive(Default)]
struct AppState {
    persistent_state: Option<AppPersistentStateManager>,
    main_window: Option<MainWindowState>,
    data_file_manager_window: Option<DataFileManager>,
    windows: Windows,
}

#[derive(Debug)]
enum AppMessage {
    None,
    Start,
    Started((AppPersistentStateManager, Option<Loader>, EarthMap)),
    WindowOpened(Window),
    WindowClosed(Window),
    WindowIdClosed(window::Id),
    MainWindowMessage(MainWindowMessage),
    DataFileManagerMessage(DataFileManagerMessage),
    LoaderUpdated((Loader, PathBuf)),
    Error(String),
}

fn update(state: &mut AppState, message: AppMessage) -> Task<AppMessage> {
    match message {
        AppMessage::Start => startup(),
        AppMessage::Started((persistent_state, loader, earth_map)) => {
            state.persistent_state = Some(persistent_state);
            let close_start_window_task = state.windows.get_starting_window().close();
            let main_window = MainWindowState::new(loader, earth_map);
            state.main_window = Some(main_window);
            close_start_window_task.chain(Window::open_main_window())
        }
        AppMessage::WindowOpened(window) => {
            state.windows.set_window(window);
            Task::none()
        }
        AppMessage::WindowClosed(window) => {
            state.windows.unset_window(window);
            match window {
                Window::Main(_) => iced::exit(),
                Window::DataFileManager(_) => {
                    state.data_file_manager_window = None;
                    Task::none()
                }
                _ => Task::none(),
            }
        }
        AppMessage::WindowIdClosed(id) => {
            Task::done(AppMessage::WindowClosed(state.windows.which_window(id)))
        }
        AppMessage::MainWindowMessage(main_window_message) => {
            if main_window_message.is_open_data_file_manager() {
                match state.windows.get_data_file_manager_window() {
                    w @ Window::DataFileManager(_) => w.focus(),
                    Window::None => {
                        let (task, _id) = Window::open_data_file_manager_window();
                        state.data_file_manager_window = Some(DataFileManager::new(
                            state
                                .persistent_state
                                .as_ref()
                                .and_then(|p| p.config().data_dir.clone()),
                        ));
                        task
                    }
                    _ => Task::none(),
                }
            } else {
                state
                    .main_window
                    .as_mut()
                    .map(|m| {
                        m.update(main_window_message)
                            .map(AppMessage::MainWindowMessage)
                    })
                    .unwrap_or(Task::none())
            }
        }
        AppMessage::DataFileManagerMessage(message) => {
            let refresh_task = if let Some(path) = message.applied_parquet_dir() {
                refresh_parquet_dir(path)
            } else {
                Task::none()
            };
            let data_file_manager_task = state
                .data_file_manager_window
                .as_mut()
                .map(|m| m.update(message).map(AppMessage::DataFileManagerMessage))
                .unwrap_or(Task::none());
            Task::batch([refresh_task, data_file_manager_task])
        }
        AppMessage::LoaderUpdated((loader, path)) => {
            let task_state = match state.persistent_state.as_mut() {
                Some(p) => match p.update_data_dir(path) {
                    Ok(_) => Task::none(),
                    Err(e) => Task::done(AppMessage::Error(e.to_string())),
                },
                None => Task::none(),
            };
            let update_loader_task = state
                .main_window
                .as_mut()
                .map(|m| m.update_loader(loader).map(AppMessage::MainWindowMessage))
                .unwrap_or(Task::none());
            Task::batch([task_state, update_loader_task])
        }
        AppMessage::Error(error) => {
            // TODO: handle errors by throwing up a window for the user.
            panic!("{}", error);
        }
        AppMessage::None => Task::none(),
    }
}

fn view(state: &AppState, window: window::Id) -> Element<'_, AppMessage> {
    let window = state.windows.which_window(window);
    match window {
        Window::Starting(_) => view_starting_window(state),
        Window::Main(_) => view_main_window(state),
        Window::DataFileManager(_) => view_data_file_manager_window(state),
        _ => text("Unknown window").into(),
    }
}

fn view_starting_window(_: &AppState) -> Element<'_, AppMessage> {
    let logo = svg(iced::widget::svg::Handle::from_memory(Assets::logo_svg()))
        .width(Length::Fill)
        .height(Length::Fill);

    container(logo)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding([10, 10])
        .into()
}

fn view_main_window(state: &AppState) -> Element<'_, AppMessage> {
    state
        .main_window
        .as_ref()
        .map(|m| m.view().map(AppMessage::MainWindowMessage))
        .unwrap_or(text("Main...").into())
}

fn view_data_file_manager_window(state: &AppState) -> Element<'_, AppMessage> {
    state
        .data_file_manager_window
        .as_ref()
        .map(|m| m.view().map(AppMessage::DataFileManagerMessage))
        .unwrap_or(text("Data file manager...").into())
}

fn refresh_parquet_dir(path: &Path) -> Task<AppMessage> {
    let path = path.to_path_buf();
    let fut = async move {
        let l = Loader::new(&path, Default::default()).await;
        (l, path)
    };
    Task::future(fut).map(|result| match result {
        (Ok(loader), path) => AppMessage::LoaderUpdated((loader, path)),
        (Err(e), _) => AppMessage::Error(e.to_string()),
    })
}

fn startup() -> Task<AppMessage> {
    Task::future(AppPersistentStateManager::open()).then(|persistent_state| {
        let (persistent_state, saving_task) = match persistent_state {
            Ok((persistent_state, saving_task)) => (persistent_state, saving_task),
            Err(e) => return Task::done(AppMessage::Error(e.to_string())),
        };
        let saving_task = saving_task.map(|_result| match _result {
            Ok(_) => AppMessage::None,
            Err(e) => AppMessage::Error(e.to_string()),
        });
        let config = persistent_state.config();
        let lattice = match Lattice::from_bytes(&Assets::lattice_hashes()) {
            Ok(lattice) => lattice,
            Err(e) => return Task::done(AppMessage::Error(e.to_string())),
        };
        let lattice = Arc::new(lattice);
        let earth_map = match EarthMap::new(
            Default::default(),
            lattice.clone(),
            Assets::earth_map_texture(),
        ) {
            Ok(earth_map) => earth_map,
            Err(e) => return Task::done(AppMessage::Error(e.to_string())),
        };

        let task_loader = match config.data_dir.clone() {
            Some(dd) => {
                let fut = async move {
                    match Loader::new(&dd, Default::default()).await {
                        Ok(loader) => {
                            AppMessage::Started((persistent_state, Some(loader), earth_map))
                        }
                        Err(e) => AppMessage::Error(e.to_string()),
                    }
                };
                Task::future(fut)
            }
            None => Task::done(AppMessage::Started((persistent_state, None, earth_map))),
        };

        Task::batch([saving_task, task_loader])
    })
}

#[cfg(target_os = "linux")]
fn platform_specific() -> window::settings::PlatformSpecific {
    window::settings::PlatformSpecific {
        // TODO: Change the id here
        application_id: "historical-marine-weather".to_string(),
        ..Default::default()
    }
}

#[cfg(not(target_os = "linux"))]
fn platform_specific() -> window::settings::PlatformSpecific {
    Default::default()
}

fn window_settings() -> window::Settings {
    window::Settings {
        icon: window::icon::from_file_data(Assets::logo_png().as_ref(), None).ok(),
        platform_specific: platform_specific(),
        ..Default::default()
    }
}

#[derive(Default, Clone, Copy, Debug)]
struct Windows([Window; 3]);

#[derive(Default, Clone, Copy, Debug)]
enum Window {
    Starting(window::Id),
    Main(window::Id),
    DataFileManager(window::Id),
    #[default]
    None,
}

impl Window {
    fn id(&self) -> Option<window::Id> {
        match self {
            Window::Starting(id) => Some(*id),
            Window::Main(id) => Some(*id),
            Window::DataFileManager(id) => Some(*id),
            _ => None,
        }
    }

    fn title(&self) -> String {
        match self {
            Window::Starting(_) => "Starting...".to_string(),
            Window::Main(_) => "Historical Marine Weather".to_string(),
            Window::DataFileManager(_) => "Data Manager".to_string(),
            _ => "Unknown Window".to_string(),
        }
    }

    fn open_starting_window() -> Task<AppMessage> {
        let (_, task) = iced::window::open(iced::window::Settings {
            decorations: false,
            resizable: false,
            position: iced::window::Position::Centered,
            size: Size::new(256., 256.),
            ..window_settings()
        });
        task.map(move |id| AppMessage::WindowOpened(Window::Starting(id)))
    }

    fn open_main_window() -> Task<AppMessage> {
        let (_, task) = iced::window::open(iced::window::Settings {
            size: Size::new(1024., 512. + 128.),
            min_size: Some(Size::new(1024., 512. + 128.)),
            ..window_settings()
        });
        task.map(move |id| AppMessage::WindowOpened(Window::Main(id)))
    }

    fn open_data_file_manager_window() -> (Task<AppMessage>, window::Id) {
        let (id, task) = iced::window::open(iced::window::Settings {
            size: Size::new(1024. - 256., 512.),
            min_size: Some(Size::new(1024. - 256., 512.)),
            ..window_settings()
        });
        (
            task.map(move |id| AppMessage::WindowOpened(Window::DataFileManager(id))),
            id,
        )
    }

    fn focus(&self) -> Task<AppMessage> {
        if let Some(id) = self.id() {
            iced::window::gain_focus(id)
        } else {
            Task::none()
        }
    }

    fn close(self) -> Task<AppMessage> {
        if let Some(id) = self.id() {
            iced::window::close(id)
        } else {
            Task::none()
        }
    }
}

impl Windows {
    fn set_window(&mut self, window: Window) {
        match window {
            w @ Window::Starting(_) => self.0[0] = w,
            w @ Window::Main(_) => self.0[1] = w,
            w @ Window::DataFileManager(_) => self.0[2] = w,
            _ => (),
        }
    }

    fn unset_window(&mut self, window: Window) {
        match window {
            Window::Starting(_) => self.0[0] = Window::None,
            Window::Main(_) => self.0[1] = Window::None,
            Window::DataFileManager(_) => self.0[2] = Window::None,
            _ => (),
        }
    }

    fn which_window(&self, id: window::Id) -> Window {
        for w in self.0.iter() {
            if w.id() == Some(id) {
                return *w;
            }
        }
        Window::None
    }

    fn get_starting_window(&self) -> Window {
        self.0.first().copied().unwrap_or(Window::None)
    }

    fn get_data_file_manager_window(&self) -> Window {
        self.0.get(2).copied().unwrap_or(Window::None)
    }
}

fn subscription(state: &AppState) -> Subscription<AppMessage> {
    let data_file_manager_subscription = state
        .data_file_manager_window
        .as_ref()
        .map(|manager| {
            manager
                .subscription()
                .map(AppMessage::DataFileManagerMessage)
        })
        .unwrap_or_else(Subscription::none);

    Subscription::batch([
        iced::window::close_events().map(AppMessage::WindowIdClosed),
        data_file_manager_subscription,
    ])
}

fn title(state: &AppState, window: window::Id) -> String {
    state.windows.which_window(window).title()
}

fn boot() -> (AppState, Task<AppMessage>) {
    (
        AppState::default(),
        Window::open_starting_window().chain(Task::done(AppMessage::Start)),
    )
}

pub fn run() {
    let mut app = iced::daemon(boot, update, view)
        .antialiasing(true)
        .title(title)
        .subscription(subscription);
    for font in Assets::fonts() {
        app = app.font(font);
    }
    app.run().unwrap();
}
