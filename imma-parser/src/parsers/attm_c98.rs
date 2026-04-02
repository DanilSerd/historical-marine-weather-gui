use nom::{
    IResult, Parser,
    bytes::complete::take,
    combinator::{all_consuming, verify},
    error::context,
};
use tinystr::TinyAsciiStr;

use crate::attm_types::{ReleaseNumber, UIDA_ATTACHMENT_ID, UidaAttachment};

use super::{error::Error, generic};

fn parse_uid(input: &[u8]) -> IResult<&[u8], TinyAsciiStr<6>, Error<&[u8]>> {
    let (rest, uid) = verify(take(6u8), |uid: &[u8]| {
        uid.iter().all(|byte| byte.is_ascii_alphanumeric())
    })
    .parse(input)?;

    let uid = TinyAsciiStr::try_from_utf8(uid).map_err(|_| Error::err(uid, "invalid ascii uid"))?;

    Ok((rest, uid))
}

pub(super) fn parse(input: &[u8]) -> IResult<&[u8], UidaAttachment, Error<&[u8]>> {
    let (rest, attm_raw) = take(15u8)(input)?;
    let (_, (_attm_id, _attm_len, uid, rn1, rn2, rn3, release_status, intermediate_reject_flag)) =
        all_consuming((
            context(
                "parse attachment id",
                verify(generic::u8(2), |value| value == &Some(UIDA_ATTACHMENT_ID)),
            ),
            context(
                "parse attachment length",
                verify(generic::u8(2), |value| value == &Some(15)),
            ),
            context("parse uid", parse_uid),
            context("parse primary release number", generic::take_base36(1)),
            context("parse secondary release number", generic::take_base36(1)),
            context("parse tertiary release number", generic::take_base36(1)),
            context("parse release status", generic::take_enum_u8(1)),
            context("parse intermediate reject flag", generic::take_enum_u8(1)),
        ))
        .parse(attm_raw)?;

    let (
        Some(primary),
        Some(secondary),
        Some(tertiary),
        Some(release_status),
        Some(intermediate_reject_flag),
    ) = (rn1, rn2, rn3, release_status, intermediate_reject_flag)
    else {
        return Err(Error::err(
            input,
            "uida attachment contains missing required fields",
        ));
    };

    Ok((
        rest,
        UidaAttachment {
            uid,
            release_number: ReleaseNumber {
                primary,
                secondary,
                tertiary,
            },
            release_status,
            intermediate_reject_flag,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::attm_types::{IntermediateRejectFlag, ReleaseStatusIndicator};

    use super::parse;

    #[test]
    fn parses_uida_attachment() {
        let (rest, parsed) = parse(b"9815CZ3CEK30021").unwrap();
        assert!(rest.is_empty());
        assert_eq!(parsed.uid.as_str(), "CZ3CEK");
        assert_eq!(parsed.release_number.primary, 3);
        assert_eq!(parsed.release_number.secondary, 0);
        assert_eq!(parsed.release_number.tertiary, 0);
        assert_eq!(parsed.release_status, ReleaseStatusIndicator::Full);
        assert_eq!(
            parsed.intermediate_reject_flag,
            IntermediateRejectFlag::Final
        );
    }
}
