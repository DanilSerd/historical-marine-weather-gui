use std::fmt::Display;

use serde::Serialize;

use imma_parser::types::{WavesDirection, WindDir};

const ALL_DIRECTIONS: [CardinalOrdinalDirection; 8] = [
    CardinalOrdinalDirection::N,
    CardinalOrdinalDirection::NE,
    CardinalOrdinalDirection::E,
    CardinalOrdinalDirection::SE,
    CardinalOrdinalDirection::S,
    CardinalOrdinalDirection::SW,
    CardinalOrdinalDirection::W,
    CardinalOrdinalDirection::NW,
];

#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash, Serialize, PartialOrd, Ord)]
pub enum CardinalOrdinalDirection {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
    /// Multiple reasons for this. e.g. Calm, Indeterminate, or confused. But always explicitly reported.
    Indeterminate,
}

impl Display for CardinalOrdinalDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CardinalOrdinalDirection::N => "North",
            CardinalOrdinalDirection::NE => "North-East",
            CardinalOrdinalDirection::E => "East",
            CardinalOrdinalDirection::SE => "South-East",
            CardinalOrdinalDirection::S => "South",
            CardinalOrdinalDirection::SW => "South-West",
            CardinalOrdinalDirection::W => "West",
            CardinalOrdinalDirection::NW => "North-West",
            CardinalOrdinalDirection::Indeterminate => "Indeterminate",
        };
        write!(f, "{}", s)
    }
}

impl CardinalOrdinalDirection {
    pub fn all_cardinal_directions() -> &'static [CardinalOrdinalDirection] {
        &ALL_DIRECTIONS[..]
    }
}

fn direction_for_degrees(degrees: u16) -> CardinalOrdinalDirection {
    let a = degrees as f32 / 45f32;
    if a.fract() < 0.5 {
        ALL_DIRECTIONS[a.floor() as usize % 8]
    } else {
        ALL_DIRECTIONS[(a.floor() as usize + 1) % 8]
    }
}
impl From<WindDir> for CardinalOrdinalDirection {
    fn from(value: WindDir) -> Self {
        match value {
            WindDir::Direction(degrees) => direction_for_degrees(degrees),
            WindDir::Calm => Self::Indeterminate,
            WindDir::Variable => Self::Indeterminate,
        }
    }
}

impl From<WavesDirection> for CardinalOrdinalDirection {
    fn from(value: WavesDirection) -> Self {
        match value {
            WavesDirection::Direction(d) => direction_for_degrees(d),
            WavesDirection::IndeterminateLow => Self::Indeterminate,
            WavesDirection::IndeterminateHigh => Self::Indeterminate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_wind_dir() {
        assert_eq!(
            CardinalOrdinalDirection::from(WindDir::Direction(0)),
            CardinalOrdinalDirection::N
        );
        assert_eq!(
            CardinalOrdinalDirection::from(WindDir::Direction(45)),
            CardinalOrdinalDirection::NE
        );
        assert_eq!(
            CardinalOrdinalDirection::from(WindDir::Direction(90)),
            CardinalOrdinalDirection::E
        );
        assert_eq!(
            CardinalOrdinalDirection::from(WindDir::Direction(135)),
            CardinalOrdinalDirection::SE
        );
        assert_eq!(
            CardinalOrdinalDirection::from(WindDir::Direction(180)),
            CardinalOrdinalDirection::S
        );
        assert_eq!(
            CardinalOrdinalDirection::from(WindDir::Direction(225)),
            CardinalOrdinalDirection::SW
        );
        assert_eq!(
            CardinalOrdinalDirection::from(WindDir::Calm),
            CardinalOrdinalDirection::Indeterminate
        );
        assert_eq!(
            CardinalOrdinalDirection::from(WindDir::Variable),
            CardinalOrdinalDirection::Indeterminate
        );
    }
}
