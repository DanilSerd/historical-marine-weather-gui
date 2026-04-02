#[derive(thiserror::Error, Debug)]
pub struct IMMAParseError<I> {
    pub input: I,
    pub context: Option<&'static str>,
    pub kind: Option<nom::error::ErrorKind>,
    pub other: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub struct Error<I>(pub Vec<IMMAParseError<I>>);

impl<I> FromIterator<IMMAParseError<I>> for Error<I> {
    fn from_iter<T: IntoIterator<Item = IMMAParseError<I>>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl<I> Error<I> {
    pub fn err(input: I, other: impl AsRef<str>) -> nom::Err<Self> {
        nom::Err::Error(Self(vec![IMMAParseError {
            input,
            other: Some(other.as_ref().into()),
            kind: None,
            context: None,
        }]))
    }
}

impl<I> nom::error::ParseError<I> for Error<I> {
    fn from_error_kind(input: I, kind: nom::error::ErrorKind) -> Self {
        Self(vec![IMMAParseError {
            input,
            kind: Some(kind),
            context: None,
            other: None,
        }])
    }

    fn append(input: I, kind: nom::error::ErrorKind, mut other: Self) -> Self {
        other.0.push(IMMAParseError {
            input,
            kind: Some(kind),
            other: None,
            context: None,
        });
        other
    }
}

impl<I> nom::error::ContextError<I> for Error<I> {
    fn add_context(input: I, ctx: &'static str, mut other: Self) -> Self {
        other.0.push(IMMAParseError {
            input,
            context: Some(ctx),
            kind: None,
            other: None,
        });
        other
    }
}
