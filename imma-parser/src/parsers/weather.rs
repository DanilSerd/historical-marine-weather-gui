use nom::{IResult, Parser, bytes::complete::take, combinator::all_consuming, error::context};

use crate::types::{
    Visibility, WavePeriod, Waves, WavesDirection, Wind, WindDir, WindDirection, WindSpeed,
};

use super::{error::Error, generic};

pub(super) fn parse_wind(input: &[u8]) -> IResult<&[u8], Wind, Error<&[u8]>> {
    let (rest, i) = take(8u8)(input)?;
    let (_, (wdi, wd, wsi, ws)) = all_consuming((
        context("parse wind direction indicator", generic::take_enum_u8(1)),
        context("parse wind direction", parse_wind_direction),
        context("parse wind speed indicator", generic::take_enum_u8(1)),
        context("parse wind speed", generic::u16(3)),
    ))
    .parse(i)?;
    let direction = wd.zip(wdi).map(|(direction, indicator)| WindDirection {
        direction,
        indicator,
    });
    let speed = ws.zip(wsi).map(|(speed, indicator)| WindSpeed {
        speed: speed as f32 / 10.,
        indicator,
    });

    Ok((rest, Wind { direction, speed }))
}

pub(super) fn parse_visibility(input: &[u8]) -> IResult<&[u8], Option<Visibility>, Error<&[u8]>> {
    let (rest, i) = take(3u8)(input)?;
    let (_, (indic, vis)) = all_consuming((
        context("parse visibility indicator", generic::take_enum_u8(1)),
        context("parse visibility", generic::take_enum_u8(2)),
    ))
    .parse(i)?;
    let vis = vis.zip(indic).map(|(visibility, indicator)| Visibility {
        visibility,
        indicator,
    });

    Ok((rest, vis))
}

pub(super) fn parse_waves(input: &[u8]) -> IResult<&[u8], Waves, Error<&[u8]>> {
    let (rest, i) = take(6u8)(input)?;
    let (_, (wd, wp, wh)) = all_consuming((
        context("parse wave direction", generic::u16(2)),
        context("parse wave period", generic::u8(2)),
        context("parse wave height", generic::u8(2)),
    ))
    .parse(i)?;

    let wd = match wd {
        Some(wd) => Some(WavesDirection::try_from(wd * 10).map_err(|e| Error::err(input, e))?),
        None => None,
    };

    let wp = match wp {
        Some(wp) => Some(WavePeriod::try_from(wp).map_err(|e| Error::err(input, e))?),
        None => None,
    };

    Ok((
        rest,
        Waves {
            direction: wd,
            period: wp,
            height: wh.map(|h| h as f32 / 2.),
        },
    ))
}

fn parse_wind_direction(input: &[u8]) -> IResult<&[u8], Option<WindDir>, Error<&[u8]>> {
    let (rest, wd) = generic::u16(3)(input)?;
    let wd = match wd {
        Some(wd) => Some(WindDir::try_from(wd).map_err(|e| Error::err(input, e))?),
        None => None,
    };

    Ok((rest, wd))
}
