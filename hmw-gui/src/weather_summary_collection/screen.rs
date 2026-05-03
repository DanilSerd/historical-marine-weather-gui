use iced::{
    Element, Length,
    widget::text,
    widget::{button, container, pane_grid, text::LineHeight},
};

use super::{WeatherSummaryCollectionMessage, messages::PaneMessage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    EarthMap,
    WeatherSummaryForm,
    WeatherSummaryList,
    DataDisplay,
    WeatherSummaryDetails,
    DataDisplaySidePanel,
}

#[derive(Debug, Clone)]
pub struct MainScreenPaneState(pane_grid::State<Pane>);

#[derive(Debug, Clone)]
pub struct WindrosePaneState(pane_grid::State<Pane>);

impl MainScreenPaneState {
    pub fn new() -> Self {
        let panes = pane_grid::State::with_configuration(pane_grid::Configuration::Split {
            axis: pane_grid::Axis::Vertical,
            ratio: 0.6,
            a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: 0.6,
                a: Box::new(pane_grid::Configuration::Pane(Pane::EarthMap)),
                b: Box::new(pane_grid::Configuration::Pane(Pane::WeatherSummaryDetails)),
            }),
            b: Box::new(pane_grid::Configuration::Pane(Pane::WeatherSummaryList)),
        });
        Self(panes)
    }

    pub fn open_form_if_not_open(&mut self) {
        if self.form_open() {
            return;
        }
        let (p, _) = self
            .0
            .iter()
            .find(|(_, p)| **p == Pane::WeatherSummaryList)
            .expect("earth map is always open");
        self.0
            .split(pane_grid::Axis::Horizontal, *p, Pane::WeatherSummaryForm);
    }

    pub fn form_open(&self) -> bool {
        self.0.iter().any(|(_, p)| *p == Pane::WeatherSummaryForm)
    }

    pub fn state(&self) -> &pane_grid::State<Pane> {
        &self.0
    }

    pub fn close_form(&mut self) {
        let p = self
            .0
            .iter()
            .find(|(_, p)| **p == Pane::WeatherSummaryForm)
            .map(|(p, _)| p)
            .cloned();
        if let Some(p) = p {
            self.0.close(p);
        }
    }
}

impl WindrosePaneState {
    pub fn new() -> Self {
        let panes = pane_grid::State::with_configuration(pane_grid::Configuration::Split {
            axis: pane_grid::Axis::Vertical,
            ratio: 0.6,
            a: Box::new(pane_grid::Configuration::Pane(Pane::DataDisplay)),
            b: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: 0.5,
                a: Box::new(pane_grid::Configuration::Pane(Pane::DataDisplaySidePanel)),
                b: Box::new(pane_grid::Configuration::Pane(Pane::WeatherSummaryDetails)),
            }),
        });
        Self(panes)
    }

    pub fn state(&self) -> &pane_grid::State<Pane> {
        &self.0
    }
}

impl Default for MainScreenPaneState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for WindrosePaneState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn content(
    element: Element<'_, WeatherSummaryCollectionMessage>,
    title: impl ToString,
    close_message: Option<PaneMessage>,
) -> pane_grid::Content<'_, WeatherSummaryCollectionMessage> {
    let mut title_bar = pane_grid::TitleBar::new(
        container(
            text(title.to_string())
                .size(15.)
                .line_height(LineHeight::Relative(1.)),
        )
        .padding([3, 6]),
    )
    .style(style::title_bar_active);
    if let Some(close_message) = close_message {
        title_bar = title_bar.controls(pane_grid::Controls::new(
            button(
                container(text("X").size(14).line_height(LineHeight::Relative(1.)))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
            .style(button::danger)
            .padding(0)
            .width(Length::Fixed(26.0))
            .height(Length::Fixed(26.0))
            .on_press(WeatherSummaryCollectionMessage::PaneMessage(close_message)),
        ));
    }
    pane_grid::Content::new(container(element).padding(5))
        .style(style::pane_active)
        .title_bar(title_bar)
}

pub mod style {
    use iced::widget::container;
    use iced::{Border, Theme};

    pub fn pane_active(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            background: Some(palette.background.weak.color.into()),
            border: Border {
                width: 2.0,
                color: palette.background.strong.color,
                ..Border::default()
            },
            ..Default::default()
        }
    }

    pub fn title_bar_active(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            text_color: Some(palette.background.strong.text),
            background: Some(palette.background.strong.color.into()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Screen {
    pub main: MainScreenPaneState,
    pub windrose: WindrosePaneState,
    pub selection: ScreenSelection,
}

impl Screen {
    pub fn open_form(&mut self) {
        self.main.open_form_if_not_open();
    }

    pub fn resize(&mut self, event: &pane_grid::ResizeEvent) {
        match self.selection {
            ScreenSelection::Main => {
                self.main.0.resize(event.split, event.ratio);
            }
            ScreenSelection::Windrose => {
                self.windrose.0.resize(event.split, event.ratio);
            }
        }
    }

    pub fn toggle_screen(&mut self) {
        match self.selection {
            ScreenSelection::Main => self.selection = ScreenSelection::Windrose,
            ScreenSelection::Windrose => self.selection = ScreenSelection::Main,
        }
    }

    pub fn close_form(&mut self) {
        self.main.close_form();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreenSelection {
    #[default]
    Main,
    Windrose,
}
