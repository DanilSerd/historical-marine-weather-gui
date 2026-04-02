use nom::{IResult, Parser, branch::alt, bytes::complete::take, combinator::rest, error::context};

use super::error::Error;
use crate::types::{IMMA_ATTACHMENTS_COUNT, IMMAAttachment, IMMAAttachments};

use super::generic;

fn take_attm(input: &[u8]) -> IResult<&[u8], IMMAAttachment<'_>, Error<&[u8]>> {
    let (_, i) = take(4u8)(input)?;

    let (_, (id, len)): (_, (_, Option<u32>)) = (
        context(
            "parse attm id",
            alt((generic::u8(2), generic::take_base36(2))),
        ),
        context(
            "parse attm length",
            alt((generic::u32(2), generic::take_base36(2))),
        ),
    )
        .parse(i)?;

    let (id, len) = match (id, len) {
        (Some(id), Some(len)) => (id, len),
        id_len => {
            return Err(Error::err(
                i,
                format!("{:?}, missing id or length for attachment", id_len),
            ));
        }
    };

    let (rest, attachment) = if len == 0 {
        rest(input)
    } else {
        take(len)(input)
    }?;

    Ok((rest, IMMAAttachment { id, attachment }))
}

/// Take all attachments. Non Streaming.
pub(super) fn take_attachments(input: &[u8]) -> IResult<&[u8], IMMAAttachments<'_>, Error<&[u8]>> {
    let mut rest = input;
    let mut attms: IMMAAttachments = [None; IMMA_ATTACHMENTS_COUNT];
    for a in attms.iter_mut() {
        let (r, attm) = take_attm(rest)?;
        rest = r;
        let _ = a.insert(attm);
        if rest.is_empty() {
            break;
        }
    }
    Ok((rest, attms))
}
