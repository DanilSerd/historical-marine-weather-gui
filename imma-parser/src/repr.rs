macro_rules! impl_repr_u8 {
    ($($t:ty)+) => {
        $(
            impl From<$t> for u8 {
                fn from(value: $t) -> Self {
                    value as u8
                }
            }

            impl TryFrom<u8> for $t {
                type Error = &'static str;

                fn try_from(value: u8) -> Result<Self, Self::Error> {
                    Self::from_repr(value).ok_or("unknown u8 representation")
                }
            }
        )+
    }
}

pub(crate) use impl_repr_u8;
