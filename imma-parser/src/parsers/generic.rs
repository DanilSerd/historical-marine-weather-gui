use nom::{
    IResult, Parser,
    combinator::{all_consuming, map_parser},
    error::ParseError,
    sequence::preceded,
};

macro_rules! take_char_ints {
    ($($t:tt)+) => {
        $(
            /// Will return a parser to take count of bytes and parse those bytes as characters into a number.
            /// Will make sure to strip any leading whitespace, and all input is consumed.
            pub(super) fn $t<'a, E>(
                count: u8,
            ) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], Option<$t>, E>
            where
                E: ParseError<&'a [u8]>,
            {
                move |input| {
                    let (rest, chars) = map_parser(
                        nom::bytes::complete::take(count),
                        preceded(nom::character::complete::multispace0, nom::combinator::rest),
                    ).parse(input)?;
                    let res = match chars.len() {
                        0 => None,
                        _ => Some(all_consuming(nom::character::complete::$t).parse(chars)?.1),
                    };
                    Ok((rest, res))
                }
            }
        )+
    }
}

take_char_ints! {u8 u16 i32 u32}

pub(super) fn take_base36<'a, E, U>(
    count: u8,
) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], Option<U>, E>
where
    E: ParseError<&'a [u8]>,
    U: TryFrom<u128>,
{
    move |input| {
        let (rest, chars) = map_parser(
            nom::bytes::complete::take(count),
            preceded(nom::character::complete::multispace0, nom::combinator::rest),
        )
        .parse(input)?;

        if chars.is_empty() {
            return Ok((rest, None));
        }

        let decoded = decode_base36(chars)
            .ok_or(nom::Err::Error(E::from_error_kind(
                chars,
                nom::error::ErrorKind::Digit,
            )))?
            .try_into()
            .map_err(|_| {
                nom::Err::Error(E::from_error_kind(chars, nom::error::ErrorKind::Digit))
            })?;

        Ok((rest, Some(decoded)))
    }
}

fn decode_base36(input: &[u8]) -> Option<u128> {
    input
        .iter()
        .rev()
        .enumerate()
        .try_fold(0u128, |acc, (i, c)| {
            char::from(*c).to_digit(36).and_then(|val| {
                let p = 36u128.checked_pow(i as u32)?;
                acc.checked_add((val as u128) * p)
            })
        })
}

pub(super) fn take_enum_u8<'a, E, R>(
    count: u8,
) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], Option<R>, E>
where
    E: ParseError<&'a [u8]>,
    R: TryFrom<u8>,
{
    move |input| {
        let (rest, num) = u8(count)(input)?;
        let num = num.map(|n| R::try_from(n)).transpose().map_err(|_| {
            nom::Err::Error(E::from_error_kind(input, nom::error::ErrorKind::Digit))
        })?;
        Ok((rest, num))
    }
}

#[test]
fn test_base_36() {
    let under_test = b"65";
    let (_, r) = take_base36::<nom::error::Error<&[u8]>, u8>(2)(under_test).unwrap();
    dbg!(r);
}
