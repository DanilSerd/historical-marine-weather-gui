use std::sync::Arc;

use crate::types::*;
use arrow::{
    array::{Array, ArrayBuilder, RecordBatch},
    datatypes::{Field, Schema},
};
use arrow_convert::serialize::ArrowSerialize;
use arrow_convert::{
    deserialize::{ArrowDeserialize, arrow_array_deserialize_iterator_as_type},
    field::ArrowField,
};
use itertools::izip;

macro_rules! define_new_serde {
    ($struct_name:ident { $( $field:ident : $builder:ty ),+ }) => {
        pub struct $struct_name {
            schema: Arc<Schema>,
            $(
                $field: <Option<$builder> as ArrowSerialize>::ArrayBuilderType,
            )+
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $struct_name {
            pub fn new() -> Self {
                Self {
                    schema: Self::build_schema(),
                    $(
                        $field: <Option<$builder> as ArrowSerialize>::new_array(),
                    )+
                }
            }

            pub fn build_schema() -> Arc<Schema> {
                let fields = vec![$(Field::new(
                    stringify!($field),
                    <Option<$builder> as ArrowField>::data_type(),
                    true,
                )),+];
                Arc::new(Schema::new(fields))
            }

            /// Append to the array builders. [`Self::serialize`] must be called once you want to build the [`RecordBatch`].
            pub fn append(&mut self, record: &IMMARecord) {
                $(
                    <Option<$builder> as ArrowSerialize>::arrow_serialize(
                        &record.$field,
                        &mut self.$field,
                    ).expect("can serialize");
                )+
            }

            /// Returns an iterator that yields [`IMMARecord`]. Each field will be populated if the [`RecordBatch`] contains the column,
            /// and it's the correct schema, otherwise it will be None. This allows for partial deserialization if you have a [`RecordBatch`]
            /// with just a few columns you care about.
            pub fn deserialize<'a>(batch: &'a RecordBatch) -> impl Iterator<Item = IMMARecord> + 'a {
                $(

                    let $field = deserialize_column::<$builder>(batch, stringify!($field));
                )+
                let iter = izip!($($field),+);
                iter.map(|($($field),+)| IMMARecord {
                    $($field),+,
                    ..Default::default()
                })
            }

            /// Finish all the array builders and return a [`RecordBatch`].
            /// [`Self`] will be reset so it can be used again for building a new batch.
            pub fn serialize(&mut self) -> RecordBatch {
                let arrays: Vec<Arc<dyn Array>> = vec![$(Arc::new(self.$field.finish())),+];
                // We know all the types are correct so safe to unwrap
                RecordBatch::try_new(self.schema.clone(), arrays).expect("can build record batch")
            }
        }
    };
}

define_new_serde!(ArrowSerde {
    date_time: DateTime,
    position: Position,
    meta: Meta,
    ship: Ship,
    wind: Wind,
    visibility: Visibility
});

impl ArrowSerde {
    pub fn len(&self) -> usize {
        self.date_time.len()
    }

    pub fn schema(&self) -> Arc<Schema> {
        self.schema.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.date_time.is_empty()
    }
}

fn deserialize_column<'a, T>(
    batch: &'a RecordBatch,
    column: &'static str,
) -> impl Iterator<Item = Option<<T as ArrowField>::Type>> + 'a
where
    T: ArrowDeserialize + 'static,
{
    let rows = batch.num_rows();
    let column = batch.column_by_name(column);
    let iter =
        column.and_then(|c| arrow_array_deserialize_iterator_as_type::<_, Option<T>>(c).ok());

    let iter: Box<dyn Iterator<Item = _>> = if let Some(dt) = iter {
        Box::new(dt)
    } else {
        Box::new((0..rows).map(|_| None))
    };
    iter
}

#[cfg(test)]
mod tests {

    use super::ArrowSerde;

    #[test]
    fn test_serde() {
        let mut serde = ArrowSerde::new();
        let under_test = crate::parsers::test::build_test();
        serde.append(&under_test);
        let rb = serde.serialize();
        assert_eq!(rb.num_rows(), 1);
        assert_eq!(rb.num_columns(), 6);
        let iter = ArrowSerde::deserialize(&rb);
        let reserded_records: Vec<_> = iter.collect();
        assert_eq!(reserded_records.len(), 1);
        assert!(reserded_records[0].date_time.is_some());
        assert!(reserded_records[0].position.is_some());
        assert!(reserded_records[0].meta.is_some());
        assert!(reserded_records[0].ship.is_some());
        assert!(reserded_records[0].wind.is_some());
    }

    #[test]
    fn test_serde_partial() {
        let mut serde = ArrowSerde::new();
        let under_test = crate::parsers::test::build_test();
        serde.append(&under_test);
        let mut rb = serde.serialize();
        let _ = rb.remove_column(0);
        let iter = ArrowSerde::deserialize(&rb);
        let reserded_records: Vec<_> = iter.collect();
        assert!(reserded_records[0].date_time.is_none());
        assert!(reserded_records[0].position.is_some());
        assert!(reserded_records[0].meta.is_some());
        assert!(reserded_records[0].ship.is_some());
        assert!(reserded_records[0].wind.is_some());
    }
}
