use arrow::{
    array::{ArrowPrimitiveType, TimestampSecondArray, TimestampSecondBuilder},
    datatypes::{DataType, TimestampSecondType},
};
use arrow_convert::{
    deserialize::{ArrowArrayIterable, ArrowDeserialize},
    field::ArrowField,
    serialize::ArrowSerialize,
};
use chrono::{NaiveTime, Timelike};

use crate::{
    attm_types::{IntermediateRejectFlag, ReleaseStatusIndicator},
    types::*,
};

macro_rules! arrow_repr {
    ($t:ty => ($($for_t:ty),+)) => {
        $(

        impl ArrowField for $for_t {
            type Type = Self;

            fn data_type() -> arrow::datatypes::DataType {
                <$t as ArrowField>::data_type()
            }
        }

        impl ArrowSerialize for $for_t {
            type ArrayBuilderType = <$t as ArrowSerialize>::ArrayBuilderType;

            fn new_array() -> Self::ArrayBuilderType {
                <$t as ArrowSerialize>::new_array()
            }

            fn arrow_serialize(
                v: &<Self as arrow_convert::field::ArrowField>::Type,
                array: &mut Self::ArrayBuilderType,
            ) -> arrow::error::Result<()> {
                <$t as ArrowSerialize>::arrow_serialize(&(*v).into(), array)
            }
        }

        impl ArrowDeserialize for $for_t {
            type ArrayType = <$t as ArrowDeserialize>::ArrayType;

            fn arrow_deserialize(
                v: <Self::ArrayType as ArrowArrayIterable>::Item<'_>,
            ) -> Option<<Self as ArrowField>::Type> {
                v.map(|v| v.try_into()).transpose().ok().flatten()
            }
        }
        )+
    }
}

arrow_repr! {u8 => (TimeIndicator,PositionIndicator,ShipCourse,ShipIDIndicator,WindDirectionIndicator,WindSpeedIndicator,Vis,VisIndicator,WavePeriod, ReleaseStatusIndicator, IntermediateRejectFlag)}
arrow_repr! {u16 => (WindDir)}
arrow_repr! {u16 => (WavesDirection)}

// TODO: Introduce Time32Second to arrow_convert
impl ArrowField for NaiveTimeWrapper {
    type Type = Self;

    fn data_type() -> DataType {
        TimestampSecondType::DATA_TYPE
    }
}

impl ArrowSerialize for NaiveTimeWrapper {
    type ArrayBuilderType = TimestampSecondBuilder;

    fn new_array() -> Self::ArrayBuilderType {
        TimestampSecondBuilder::new()
    }

    fn arrow_serialize(
        v: &<Self as ArrowField>::Type,
        array: &mut Self::ArrayBuilderType,
    ) -> arrow::error::Result<()> {
        array.append_value(v.num_seconds_from_midnight() as i64);
        Ok(())
    }
}

impl ArrowDeserialize for NaiveTimeWrapper {
    type ArrayType = TimestampSecondArray;

    fn arrow_deserialize(
        v: <Self::ArrayType as ArrowArrayIterable>::Item<'_>,
    ) -> Option<<Self as ArrowField>::Type> {
        Some(NaiveTime::from_num_seconds_from_midnight_opt(v? as u32, 0)?.into())
    }
}
