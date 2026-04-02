use nom::{IResult, Parser, bytes::complete::take, combinator::all_consuming, error::context};
use tinystr::TinyAsciiStr;

use crate::types::{Ship, ShipID};

use super::{error::Error, generic};

pub(super) fn parse(input: &[u8]) -> IResult<&[u8], Ship, Error<&[u8]>> {
    let (remaining, i) = take(17u8)(input)?;
    let (_, (course, speed, national_source, id_indicator, id, country_code)) = all_consuming((
        context("parse course", generic::take_enum_u8(1)),
        context("parse speed", generic::u8(1)),
        context("parse national source", generic::u8(2)),
        context("parse id indicator", generic::take_enum_u8(2)),
        context("parse id", take(9u8)),
        context("parse country code", take(2u8)),
    ))
    .parse(i)?;

    let country_code =
        (country_code != b"  ").then_some(TinyAsciiStr::from_utf8_lossy(country_code, b' '));
    let id = (id != b"         ").then_some(TinyAsciiStr::from_utf8_lossy(id, b' '));
    let id = id.zip(id_indicator).map(|(i, ind)| ShipID {
        id: i,
        indicator: ind,
    });
    Ok((
        remaining,
        Ship {
            course,
            national_source,
            speed,
            id,
            country_code,
        },
    ))
}
