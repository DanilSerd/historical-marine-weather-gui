use arrow::{
    array::{ArrayRef, RecordBatch, StructArray, as_struct_array, new_null_array},
    compute::{SortColumn, take},
    datatypes::{DataType, Schema, SchemaRef},
};
use arrow_convert::{deserialize::ArrowDeserialize, field::ArrowField, serialize::ArrowSerialize};
use datafusion::prelude::DataFrame;
use parquet::errors::ParquetError;
use std::{fmt::Debug, sync::Arc};

/// Implementing this trait allows to store the type as parquet and query it.
pub trait AsParquet: ArrowSerialize + ArrowDeserialize + Send + Sync + 'static {
    type Partition: ArrowField + std::hash::Hash + Eq + Clone + Debug;
    type Predicate;

    /// Name of the fields to sort by when writing to parquet file. Every row group will be sorted by this.
    const SORT_FIELDS: &[&str];

    /// Apply some partitioning. The underlying file will have row groups partitioned by this.
    fn partition(&self) -> Self::Partition;

    /// Apply any filter/sort/projection. The resulting dataframe *MUST* produce record batches that are compatible with Self.
    fn read(
        predicate_projection: &Self::Predicate,
        df: DataFrame,
    ) -> Result<DataFrame, datafusion::error::DataFusionError>;

    /// This is needed because [`ArrowField`] has the associated type which is what is serialized.
    /// Usually this should just return `&self`.
    fn underlying_type(&self) -> &Self::Type;

    /// Schema of the data.
    fn schema() -> parquet::errors::Result<SchemaRef> {
        let schema = Arc::new(match Self::data_type() {
            DataType::Struct(f) => Schema::new(f),
            _ => {
                return Err(ParquetError::External(
                    "only struct data type supported".into(),
                ));
            }
        });
        Ok(schema)
    }

    /// Sorts the struct array and build it as a flat record batch to store to parquet.
    fn build_batch(struct_array: ArrayRef) -> Result<RecordBatch, ParquetError> {
        let sorted_array = {
            let struct_array = as_struct_array(&struct_array);
            let mut sort_columns = Vec::with_capacity(Self::SORT_FIELDS.len());
            for sort_field in Self::SORT_FIELDS {
                let array = struct_array
                    .column_by_name(sort_field)
                    .ok_or(ParquetError::External(
                        format!("Column {} doesn't exist for sorting", sort_field).into(),
                    ))?
                    .clone();
                sort_columns.push(SortColumn {
                    values: array,
                    options: None,
                });
            }
            let si = arrow::compute::lexsort_to_indices(&sort_columns, None)?;
            take(struct_array, &si, None)?
        };

        let struct_array = as_struct_array(&sorted_array);

        Ok(RecordBatch::try_new(
            Self::schema()?,
            struct_array.columns().into(),
        )?)
    }

    /// Deconstruct a flat record batch and put the columns together back into a struct array.
    /// If some columns are missing will try to re-fill those if they are nullable.
    fn build_array(record_batch: RecordBatch) -> parquet::errors::Result<ArrayRef> {
        let schema = Self::schema()?;
        let mut fields = Vec::with_capacity(schema.fields.len());
        let mut arrays = Vec::with_capacity(schema.fields.len());
        for schema_field in schema.fields.into_iter() {
            match record_batch.column_by_name(schema_field.name()) {
                Some(c) => {
                    fields.push(schema_field.clone());
                    arrays.push(c.clone());
                }
                None => {
                    if !schema_field.is_nullable() {
                        return Err(ParquetError::External(
                            format!(
                                "Field {} is not nullable but is missing from record batch",
                                schema_field
                            )
                            .into(),
                        ));
                    }
                    fields.push(schema_field.clone());
                    arrays.push(new_null_array(
                        schema_field.data_type(),
                        record_batch.num_rows(),
                    ))
                }
            }
        }
        let array = Arc::new(StructArray::new(fields.into(), arrays, None));
        Ok(array)
    }
}

pub trait DataStats {
    type Stats;

    /// Build the stats dataframe from the raw table dataframe.
    fn build(df: DataFrame) -> Result<DataFrame, datafusion::error::DataFusionError>;

    /// Process the batch and return the stats.
    fn stats(batch: RecordBatch)
    -> Result<Option<Self::Stats>, datafusion::error::DataFusionError>;
}
