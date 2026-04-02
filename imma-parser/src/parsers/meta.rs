use nom::{IResult, Parser, bytes::complete::take};

use crate::types::Meta;

use super::{Error, generic};

pub(super) fn parse(input: &[u8]) -> IResult<&[u8], Meta, Error<&[u8]>> {
    let (rest, meta_raw) = take(3u8)(input)?;
    let (_, (version, attm_count)) = (generic::u8(2), generic::take_base36(1)).parse(meta_raw)?;

    let version = version.ok_or(Error::err(input, "version missing"))?;
    let attachments_count = attm_count.ok_or(Error::err(input, "attachments count missing"))?;
    Ok((
        rest,
        Meta {
            version,
            attachments_count,
        },
    ))
}
