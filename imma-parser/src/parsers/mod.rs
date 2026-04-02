mod attm;
mod attm_c98;
mod date_time;
mod error;
mod generic;
mod meta;
mod position;
mod ship;
mod weather;

use bytes::Bytes;
pub use error::Error;
pub use error::IMMAParseError;
use nom::bytes::streaming::take;
use nom::bytes::streaming::take_until;
use nom::combinator::all_consuming;
use nom::{IResult, Parser, character::streaming::line_ending, error::context};

use crate::parsers::attm::take_attachments;
use crate::types::IMMARecord;

pub fn parse(input: &[u8]) -> IResult<&[u8], Option<IMMARecord>, Error<&[u8]>> {
    let (rest, prefix) = take(4u8)(input)?;
    if prefix == b"9815" {
        // TODO: This row links to the main record row using the uida.
        // The overall attachemnt count includes thease rows so would need to be joined
        // For now skipped
        let (rest, _) = take_until("\n")(rest)?;
        let (rest, _) = line_ending(rest)?;
        return Ok((rest, None));
    }

    let (rest, mut core) = parse_core(input)?;
    let (rest, attachment_bytes) = take_until("\n")(rest)?;
    let (rest, _) = line_ending(rest)?;

    let (_, attms) = all_consuming(take_attachments).parse(attachment_bytes)?;

    for a in attms.into_iter().take_while(Option::is_some).flatten() {
        match a.id {
            98 => {
                let (_, uida) =
                    context("parse c98 uida attachment", attm_c98::parse).parse(a.attachment)?;
                core.attm_uida = Some(uida);
            }
            _ => continue, // TODO: Ingnoring other attachaments for now
        };
    }

    if core.attm_uida.is_none() {
        Err(Error::err(input, "c98 uida attachemnt missing"))?;
    }

    Ok((rest, Some(core)))
}

fn parse_core(input: &[u8]) -> IResult<&[u8], IMMARecord, Error<&[u8]>> {
    let (rest, core) = take(108u8)(input)?;
    let (
        _,
        (
            mut date_time,
            mut position,
            meta,
            time_indicator,
            position_indicator,
            ship,
            wind,
            visibility,
            _core_rest_unparsed, // TODO: Fill in the rest
            waves,
            swell,
        ),
    ) = (
        context("parse date and time", date_time::parse),
        context("parse lat/long position", position::parse),
        context("parse version/attm count", meta::parse),
        context("parse time indicator", generic::take_enum_u8(1)),
        context("parse position indicator", generic::take_enum_u8(1)),
        context("parse ship details", ship::parse),
        context("parse wind details", weather::parse_wind),
        context("parse visibility details", weather::parse_visibility),
        nom::combinator::map(take(40u8), Bytes::copy_from_slice),
        context("parse waves details", weather::parse_waves),
        context("parse swell details", weather::parse_waves),
    )
        .parse(core)?;
    date_time.time = date_time.time.zip(time_indicator).map(|(mut t, i)| {
        t.indicator = i;
        t
    });
    position = position.zip(position_indicator).map(|(mut p, i)| {
        p.indicator = i;
        p
    });

    Ok((
        rest,
        IMMARecord {
            date_time: Some(date_time),
            position,
            meta: Some(meta),
            ship: Some(ship),
            wind: Some(wind),
            visibility,
            waves: Some(waves),
            swell: Some(swell),
            attm_uida: None,
        },
    ))
}

#[cfg(test)]
pub(crate) mod test {
    use bytes::{BufMut, BytesMut};

    use crate::attm_types::{IntermediateRejectFlag, ReleaseStatusIndicator};
    use crate::types::IMMARecord;

    use super::parse;

    pub const TEST_BYTES: &[u8] = b"1770 1 1   0 8828  3472 1405     62221     US01204  2    0            189  1831 18011 223        0           165 1813478014911 0                   17111F111A1AA1111A3AA      594                                                                              9443         82U22280   0                                                                               9441834   9815CZ3CEK30021\n";
    pub const LONG_TEST_BYTES: &[u8] = b"202411 1   0 8844 25655 1325    116401601                  10108    8               9 -47                    165  2383798172 8 0                   1CFFFCF1AAAA1AAA9AAAA     9815TN80543020199 01425546520000880400001600005500000000011900170007e80b0100050000000900000180cf0900005d00c35c82a6aca0408aaa9a9e5a9c94404040404040404040404040404040404040404040269fa2c10011045a41d33398d4ccd0c0d8d4c8e0dcc0c8c0808080805ffffc584e7f81fff2fd16080065a3797fffffffffdfffcef80037373737\n202411 1   0 8840 33330 1325    111801779                                                                    165  3186798172 7 0                   1FFFFFF1AAAAAAAAAAAAA    49815TN80533020199 0142554652000089040000160000b100000000011902170007e80a1f11040400000a00000180cf090000005d0036fc6682ae925aaaa85a6060606c404040404040404040404040404040404040404040069fa2c100110371f3a7ad20ccc0c0d4ccd0c0d8d0c8d8ccd8ccc0811ffffc383987fffff2fd16080015fffffffe01ffffdffffffe0037373737\n202411 1   0 8830 30220 1425    116401604                  10069    8               9 -20                    165  2887798172 8 0                   1CFFFCF1AAAA1AAA1AAAA      82U-2040                                                                                             9815TN80523020199 01425546520000880400001600005500000000011900170007e80b0100050000000900000180cf0900005d00c35c88a6aca0408aaa9a9e5a9cac404040404040404040404040404040404040404040269fa2c1001100eb82e9da48d4ccd0c0d8d4e0c8e4e0c0c0808080805ffffc584e7f81fff2fd16080065a79d7fffffffffdfffceaa0037373737\n";

    pub fn build_test() -> IMMARecord {
        parse(TEST_BYTES).unwrap().1.unwrap()
    }

    #[test]
    fn parses_test_bytes() {
        let record = build_test();
        let uida = record.attm_uida.expect("uida attachment parsed");

        assert_eq!(uida.uid.as_str(), "CZ3CEK");
        assert_eq!(uida.release_number.primary, 3);
        assert_eq!(uida.release_number.secondary, 0);
        assert_eq!(uida.release_number.tertiary, 0);
        assert_eq!(uida.release_status, ReleaseStatusIndicator::Full);
        assert_eq!(uida.intermediate_reject_flag, IntermediateRejectFlag::Final);
    }

    #[test]
    fn parse_multiple_test_bytes() {
        let (rest, _) = parse(LONG_TEST_BYTES).unwrap();
        let (rest, _) = parse(rest).unwrap();
        let (rest, _) = parse(rest).unwrap();
        assert!(rest.is_empty());
    }

    #[test]
    fn skips_subsidiary_records_before_main_record() {
        let mut with_subsidiary = BytesMut::new();
        with_subsidiary.put(&b"9815SUBSIDIARY\n9815ANOTHER\n"[..]);
        with_subsidiary.put(TEST_BYTES);

        let (rest, record) = parse(&with_subsidiary).unwrap();
        assert!(record.is_none());

        let (rest, record) = parse(rest).unwrap();
        assert!(record.is_none());

        assert_eq!(parse(rest).unwrap().1.unwrap(), build_test());
    }
}
