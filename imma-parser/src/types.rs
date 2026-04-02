use std::ops::Deref;

use arrow_convert::{ArrowDeserialize, ArrowField, ArrowSerialize};
use chrono::{NaiveDate, NaiveTime};
use strum_macros::FromRepr;
use tinystr::TinyAsciiStr;

use crate::attm_types::UidaAttachment;
use crate::repr::impl_repr_u8;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct IMMARecord {
    pub date_time: Option<DateTime>,
    pub position: Option<Position>,
    pub meta: Option<Meta>,
    pub ship: Option<Ship>,
    pub wind: Option<Wind>,
    pub visibility: Option<Visibility>,
    pub waves: Option<Waves>,
    pub swell: Option<Waves>,
    pub attm_uida: Option<UidaAttachment>,
}

#[derive(Debug, PartialEq, Clone, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct DateTime {
    /// Date of observation.
    pub date: Option<NaiveDate>,
    /// Time of observation.
    pub time: Option<IMMATime>,
    /// Sometimes some parts of date or time are missing or incorrect. e.g. 31 days in Sept.
    /// In those cases date and/or time will be missing and the raw parts are provided for
    /// the user of the crate to decide what to do with those.
    /// (year, month, day, hour)
    pub raw_parts: DateTimeRawParts,
}

#[derive(Debug, PartialEq, Clone, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct IMMATime {
    /// Time of observation.
    pub time: NaiveTimeWrapper,
    /// Time precision indicator. Captures the original precision at which time was recorded.
    pub indicator: TimeIndicator,
}

#[derive(Debug, PartialEq, Clone, Copy)]
// TODO: remove this wrapper once NaiveTime works with arrow_convert
pub struct NaiveTimeWrapper(pub NaiveTime);

impl Deref for NaiveTimeWrapper {
    type Target = NaiveTime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<NaiveTime> for NaiveTimeWrapper {
    fn from(value: NaiveTime) -> Self {
        NaiveTimeWrapper(value)
    }
}

#[derive(Debug, PartialEq, Clone, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct DateTimeRawParts {
    // Year.
    pub year: Option<u16>,
    // Month.
    pub month: Option<u8>,
    // Day of month.
    pub day: Option<u8>,
    // Hour as f32. e.g. `15.5f32` is 15:30.
    pub hour: Option<f32>,
}

#[derive(Debug, PartialEq, Clone, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct Position {
    /// Latitude from -90.00 to +90.00.
    pub la: f32,
    /// Longitude from -179.99 to +180.00.
    pub lo: f32,
    /// Position precision indicator. Captures the original precision at which position was recorded.
    pub indicator: PositionIndicator,
}

#[derive(FromRepr, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum TimeIndicator {
    NearestWholeHour = 0,
    HourToTenth = 1,
    HourPlusMinutes = 2,
    HighResolution = 3,
    Undefined = 255,
}

#[derive(FromRepr, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum PositionIndicator {
    DegreesAndTenths = 0,
    WholeDegrees = 1,
    MixedPrecision = 2,
    Interpolated = 3,
    DegreesAndMinutes = 4,
    HighResolution = 5,
    Other = 6,
    Undefined = 255,
}

#[derive(Debug, PartialEq, Default, Clone, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct Meta {
    pub version: u8,
    pub attachments_count: u8,
}

#[derive(Debug, PartialEq, Default, Clone, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct Ship {
    /// Ship course.
    pub course: Option<ShipCourse>,
    /// National source.
    pub national_source: Option<u8>, // TODO: parse potentially
    /// Ship speed as indicator. Higher number corresponds to higher speed. See IMMA documentation for exact mapping to knots.
    pub speed: Option<u8>, // TODO: parse the indicator
    /// Identification of the ship.
    pub id: Option<ShipID>,
    /// Country code recruiting the ship.
    pub country_code: Option<TinyAsciiStr<2>>,
}

#[derive(FromRepr, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum ShipCourse {
    Stationary = 0,
    NE = 1,
    E = 2,
    SE = 3,
    S = 4,
    SW = 5,
    W = 6,
    NW = 7,
    N = 8,
    Unknown = 9,
    Undefined = 255,
}

#[derive(Debug, PartialEq, Clone, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct ShipID {
    pub id: TinyAsciiStr<9>,
    pub indicator: ShipIDIndicator,
}

#[derive(FromRepr, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum ShipIDIndicator {
    Unknown = 0,
    Callsign = 1,
    Generic = 2,
    WMOBuoyNumber = 3,
    OtherBuoyNumber = 4,
    CmanId = 5,
    StationNameOrNumber = 6,
    OceanographicNumber = 7,
    FishingVesselID = 8,
    NationalShipNumber = 9,
    Composite = 10,
    BuoyID = 11,
    Undefined = 255,
}

#[derive(Debug, PartialEq, Clone, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct Wind {
    pub direction: Option<WindDirection>,
    pub speed: Option<WindSpeed>,
}

#[derive(Debug, PartialEq, Clone, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct WindDirection {
    pub direction: WindDir,
    pub indicator: WindDirectionIndicator,
}

#[derive(Debug, PartialEq, Clone, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct WindSpeed {
    /// Meters per second.
    pub speed: f32,
    /// Indicator for precision of the original recorded sample.
    pub indicator: WindSpeedIndicator,
}

#[derive(FromRepr, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum WindDirectionIndicator {
    Compass36 = 0,
    Compass32 = 1,
    Compass16of36 = 2,
    Compass16of32 = 3,
    Compass8 = 4,
    Compass360 = 5,
    HighResolution = 6,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum WindDir {
    Direction(u16),
    Calm,
    Variable,
}

#[derive(FromRepr, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum WindSpeedIndicator {
    MetersPerSecondEstimated = 0,
    MetersPerSecondMeasured = 1,
    EstimatedUnitsUnknown = 2,
    KnotEstimated = 3,
    KnotMeasured = 4,
    Beaufort = 5,
    EstimatedMethodUnknown = 6,
    MeasuredUnitsUnknown = 7,
    HighResolution = 8,
    KilometersPerHourMeasured = 9,
    KilometersPerHourEstimated = 10,
}

#[derive(Debug, PartialEq, Clone, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct Visibility {
    pub visibility: Vis,
    pub indicator: VisIndicator,
}

#[derive(FromRepr, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum Vis {
    LessThan50m = 90,
    At50m = 91,
    At200m = 92,
    At500m = 93,
    AtKm = 94,
    At2Km = 95,
    At4Km = 96,
    At10Km = 97,
    At20Km = 98,
    AtorMoreThan50Km = 99,
}

#[derive(FromRepr, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum VisIndicator {
    Estimated = 0,
    Measured = 1,
    FogPresent = 2,
}

#[derive(Debug, PartialEq, Clone, ArrowField, ArrowSerialize, ArrowDeserialize)]
pub struct Waves {
    /// Direction.
    pub direction: Option<WavesDirection>,
    /// Period.
    pub period: Option<WavePeriod>,
    /// Height in meters.
    pub height: Option<f32>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum WavesDirection {
    /// Direction in degrees.
    Direction(u16),
    /// Direction is indeterminate, with wave height <= 4.75 meters.
    IndeterminateLow,
    /// Direction is indeterminate, with wave height > 4.75 meters or irrespective of height.
    IndeterminateHigh,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum WavePeriod {
    /// Period in seconds.
    Period(u8),
    /// Period is indeterminate. Confused see, calm, or some other reason.
    Indeterminate,
}

pub const IMMA_ATTACHMENTS_COUNT: usize = 99;
pub type IMMAAttachments<'a> = [Option<IMMAAttachment<'a>>; IMMA_ATTACHMENTS_COUNT];

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct IMMAAttachment<'a> {
    pub id: u8,
    pub attachment: &'a [u8],
}

impl_repr_u8! {TimeIndicator PositionIndicator ShipCourse ShipIDIndicator WindDirectionIndicator WindSpeedIndicator Vis VisIndicator}

impl TryFrom<u16> for WindDir {
    type Error = &'static str;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            361 => Ok(Self::Calm),
            362 => Ok(Self::Variable),
            dir if (1..=360).contains(&dir) => Ok(Self::Direction(dir)),
            _ => Err("unknown wind direction as u16"),
        }
    }
}

impl From<WindDir> for u16 {
    fn from(value: WindDir) -> Self {
        match value {
            WindDir::Direction(d) => d,
            WindDir::Calm => 361,
            WindDir::Variable => 362,
        }
    }
}

impl From<WavesDirection> for u16 {
    fn from(value: WavesDirection) -> Self {
        match value {
            WavesDirection::Direction(d) => d,
            WavesDirection::IndeterminateLow => 370,
            WavesDirection::IndeterminateHigh => 380,
        }
    }
}

impl TryFrom<u16> for WavesDirection {
    type Error = &'static str;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            370 => Ok(Self::IndeterminateLow),
            380 => Ok(Self::IndeterminateHigh),
            dir if dir <= 360 => Ok(Self::Direction(dir)),
            _ => Err("unknown waves direction as u16"),
        }
    }
}

impl From<WavePeriod> for u8 {
    fn from(value: WavePeriod) -> Self {
        match value {
            WavePeriod::Period(p) => p,
            WavePeriod::Indeterminate => 0,
        }
    }
}

impl TryFrom<u8> for WavePeriod {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 | 99 => Ok(Self::Indeterminate),
            p if (1..=98).contains(&p) => Ok(Self::Period(p)),
            _ => Err("unknown wave period as u8"),
        }
    }
}
