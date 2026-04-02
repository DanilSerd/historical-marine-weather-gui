use chrono::{NaiveDate, NaiveTime};
use nom::{IResult, Parser};

use crate::types::{DateTime, DateTimeRawParts, IMMATime, TimeIndicator};

use super::{error::Error, generic};

pub(super) fn parse(input: &[u8]) -> IResult<&[u8], DateTime, Error<&[u8]>> {
    let (rest, raw_parts) = (
        generic::u16(4),
        generic::u8(2),
        generic::u8(2),
        generic::u16(4),
    )
        .parse(input)?;
    let date = raw_parts
        .0
        .zip(raw_parts.1)
        .zip(raw_parts.2)
        .and_then(|((y, m), d)| date(y, m, d));
    let time = raw_parts.3.and_then(time);
    let raw_time = raw_parts.3.map(|h| h as f32 / 1e2);

    Ok((
        rest,
        DateTime {
            date,
            time: time.map(|t| IMMATime {
                time: t.into(),
                indicator: TimeIndicator::Undefined,
            }),
            raw_parts: DateTimeRawParts {
                year: raw_parts.0,
                month: raw_parts.1,
                day: raw_parts.2,
                hour: raw_time,
            },
        },
    ))
}

fn date(year: u16, month: u8, day: u8) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
}

fn time(hour: u16) -> Option<NaiveTime> {
    NaiveTime::from_num_seconds_from_midnight_opt((hour as f32 / 1e2 * 60. * 60.) as u32, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let under_test = b"1880 1 22350";
        let (_, parsed) = parse(under_test).unwrap();
        assert_eq!(
            parsed,
            DateTime {
                date: NaiveDate::from_ymd_opt(1880, 1, 2),
                time: Some(IMMATime {
                    time: NaiveTime::from_hms_opt(23, 30, 0).unwrap().into(),
                    indicator: TimeIndicator::Undefined
                }),
                raw_parts: DateTimeRawParts {
                    year: Some(1880),
                    month: Some(1),
                    day: Some(2),
                    hour: Some(23.5)
                },
            }
        );
        let under_test = b"188001022350";
        let (_, parsed) = parse(under_test).unwrap();
        assert_eq!(
            parsed,
            DateTime {
                date: NaiveDate::from_ymd_opt(1880, 1, 2),
                time: Some(IMMATime {
                    time: NaiveTime::from_hms_opt(23, 30, 0).unwrap().into(),
                    indicator: TimeIndicator::Undefined
                }),
                raw_parts: DateTimeRawParts {
                    year: Some(1880),
                    month: Some(1),
                    day: Some(2),
                    hour: Some(23.5)
                },
            }
        );
        let under_test = b"1880 1 2   0";
        let (_, parsed) = parse(under_test).unwrap();
        assert_eq!(
            parsed,
            DateTime {
                date: NaiveDate::from_ymd_opt(1880, 1, 2),
                time: Some(IMMATime {
                    time: NaiveTime::from_hms_opt(0, 0, 0).unwrap().into(),
                    indicator: TimeIndicator::Undefined
                }),
                raw_parts: DateTimeRawParts {
                    year: Some(1880),
                    month: Some(1),
                    day: Some(2),
                    hour: Some(0.)
                },
            }
        );
        let under_test = b"18800102    ";
        let (_, parsed) = parse(under_test).unwrap();
        assert_eq!(
            parsed,
            DateTime {
                date: NaiveDate::from_ymd_opt(1880, 1, 2),
                time: None,
                raw_parts: DateTimeRawParts {
                    year: Some(1880),
                    month: Some(1),
                    day: Some(2),
                    hour: None,
                },
            }
        );
        let under_test = b"1880 1      ";
        let (_, parsed) = parse(under_test).unwrap();
        assert_eq!(
            parsed,
            DateTime {
                date: None,
                time: None,
                raw_parts: DateTimeRawParts {
                    year: Some(1880),
                    month: Some(1),
                    day: None,
                    hour: None,
                },
            }
        );
        let under_test = b"18t0 1 2    ";
        assert!(parse(under_test).is_err());
    }
}
