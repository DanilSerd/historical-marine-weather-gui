use iced::{Element, Length};

#[derive(Debug, Clone)]
pub struct MonthsSelector {
    pub months: Vec<chrono::Month>,
}

#[derive(Debug, Clone)]
pub struct MonthsSelectorMessage {
    pub month: chrono::Month,
}

impl MonthsSelector {
    pub fn new() -> Self {
        Self {
            months: Vec::with_capacity(12),
        }
    }

    pub fn toggle_month(&mut self, month: chrono::Month) {
        if self.months.contains(&month) {
            self.months
                .remove(self.months.iter().position(|m| *m == month).unwrap());
        } else {
            self.months.push(month);
        }
    }

    pub fn view(&self) -> Element<'_, MonthsSelectorMessage> {
        let checkboxes = ALL_MONTHS.iter().map(|month| {
            iced::widget::checkbox(self.months.contains(month))
                .label(month.name())
                .on_toggle(|_| MonthsSelectorMessage { month: *month })
                .into()
        });
        iced::widget::scrollable(iced::widget::column(checkboxes))
            .width(Length::Fill)
            .height(Length::Shrink)
            .into()
    }

    pub fn is_empty(&self) -> bool {
        self.months.is_empty()
    }
}

const ALL_MONTHS: [chrono::Month; 12] = [
    chrono::Month::January,
    chrono::Month::February,
    chrono::Month::March,
    chrono::Month::April,
    chrono::Month::May,
    chrono::Month::June,
    chrono::Month::July,
    chrono::Month::August,
    chrono::Month::September,
    chrono::Month::October,
    chrono::Month::November,
    chrono::Month::December,
];
