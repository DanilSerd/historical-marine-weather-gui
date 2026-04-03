use iced::{Element, Length, Size, widget::Scrollable};

/// Fits a list of square elements into a scrollable container.
/// Maximizes the number of squares that fit into the [`Size`] container without scrolling.
pub fn fit_square_elements_to_scrollable<'a, M: 'static>(
    elements: impl IntoIterator<Item = Element<'a, M>>,
    number_of_elements: usize,
    size: Size<f32>,
    min_square_size: f32,
) -> Scrollable<'a, M> {
    let square_size = find_largest_square_fit(size, min_square_size, number_of_elements);

    let elements_per_row = (size.width / square_size).floor() as usize;
    let elements_per_row = elements_per_row.max(1);

    let mut row = iced::widget::Row::new()
        .spacing(5)
        .width(Length::Fill)
        .height(Length::Fixed(square_size));
    let mut column: iced::widget::Column<'_, M> = iced::widget::Column::new().spacing(5);

    for (i, e) in elements.into_iter().enumerate() {
        row = row.push(e);

        if (i + 1) % elements_per_row == 0 {
            column = column.push(row);
            row = iced::widget::Row::new()
                .spacing(5)
                .width(Length::Fill)
                .height(Length::Fixed(square_size));
        }
    }
    if !number_of_elements.is_multiple_of(elements_per_row) {
        column = column.push(row);
    }
    iced::widget::scrollable(column)
}

fn find_largest_square_fit(rectangle_size: Size<f32>, min_size: f32, target_squares: usize) -> f32 {
    let mut low = min_size;
    let mut high = rectangle_size.width.min(rectangle_size.height);

    while (high - low) > 0.001 {
        let mid = (low + high) / 2.0;

        let squares_per_row = (rectangle_size.width / mid).floor() as usize;
        let rows = (rectangle_size.height / mid).floor() as usize;
        let total_squares = squares_per_row * rows;

        if total_squares < target_squares {
            high = mid;
        } else {
            low = mid;
        }
    }

    low
}

#[cfg(test)]
mod tests {
    use iced::Size;

    use super::find_largest_square_fit;

    #[test]
    fn test_find_largest_square_fit() {
        let rectangle_size = Size::new(150.0, 350.);
        let min_size = 50.0;
        let target_squares = 5;

        let best_size = find_largest_square_fit(rectangle_size, min_size, target_squares);

        assert_eq!(best_size, 75.);
    }
}
