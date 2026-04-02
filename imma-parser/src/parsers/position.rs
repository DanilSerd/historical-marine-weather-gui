use nom::{IResult, Parser, bytes::complete::take, combinator::all_consuming, error::context};

use crate::types::{Position, PositionIndicator};

use super::{error::Error, generic};

pub(super) fn parse(input: &[u8]) -> IResult<&[u8], Option<Position>, Error<&[u8]>> {
    let (rest, i) = take(11u8)(input)?;
    let (_, (lat, long)) = all_consuming((
        context("parse latitude", parse_lat),
        context("parse longitude", parse_long),
    ))
    .parse(i)?;
    let pos = lat.zip(long).map(|(la, lo)| Position {
        la,
        lo,
        indicator: PositionIndicator::Undefined,
    });

    Ok((rest, pos))
}

fn parse_lat(input: &[u8]) -> IResult<&[u8], Option<f32>, Error<&[u8]>> {
    let (rest, lat) = generic::i32(5)(input)?;
    let lat = lat
        .map(|l| {
            if !(-9_000..=9_000).contains(&l) {
                Err(Error::err(input, format!("latitude {} is not valid", l)))?
            } else {
                Ok(l)
            }
        })
        .transpose()?;
    let lat = lat.map(|l| l as f32 / 1e2);

    Ok((rest, lat))
}

fn parse_long(input: &[u8]) -> IResult<&[u8], Option<f32>, Error<&[u8]>> {
    let (rest, long) = generic::i32(6)(input)?;
    let long = long
        .map(|l| {
            if !(-17_999..=35_999).contains(&l) {
                Err(Error::err(input, format!("longitude {} is not valid", l)))?
            } else if l > 18_000 {
                Ok(l - 36_000)
            } else {
                Ok(l)
            }
        })
        .transpose()?;
    let long = long.map(|l| l as f32 / 1e2);

    Ok((rest, long))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let under_test = b" 8000  1000";
        let (_, parsed) = parse(under_test).unwrap();
        assert_eq!(
            parsed,
            Some(Position {
                la: 80.00,
                lo: 10.00,
                indicator: PositionIndicator::Undefined
            })
        );
        let under_test = b" 8054 19001";
        let (_, parsed) = parse(under_test).unwrap();
        assert_eq!(
            parsed,
            Some(Position {
                la: 80.54,
                lo: -169.99,
                indicator: PositionIndicator::Undefined
            })
        );
        let under_test = b"           ";
        let (_, parsed) = parse(under_test).unwrap();
        assert_eq!(parsed, None);
        let under_test = b"-1000 -1000";
        let (_, parsed) = parse(under_test).unwrap();
        assert_eq!(
            parsed,
            Some(Position {
                la: -10.,
                lo: -10.,
                indicator: PositionIndicator::Undefined
            })
        );
        let under_test = b"-9500 -1000";
        assert!(parse(under_test).is_err());
        let under_test = b"-1000-18000";
        assert!(parse(under_test).is_err());
    }
}
