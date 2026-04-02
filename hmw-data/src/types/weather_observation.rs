use hmw_geo::LatticedPoint;
use hmw_parquet::AsParquet;
use hmw_parquet::DataStats;
use hmw_parquet::arrow;
use hmw_parquet::arrow::array::UInt16Array;
use hmw_parquet::arrow::array::{Array, Int64Array};
use hmw_parquet::arrow_convert;
use hmw_parquet::datafusion::error::DataFusionError;
use hmw_parquet::datafusion::functions_aggregate::count::count_all;
use hmw_parquet::datafusion::functions_aggregate::min_max::{max, min};
use hmw_parquet::datafusion::prelude::DataFrame;
use hmw_parquet::datafusion::prelude::Expr;
use hmw_parquet::datafusion::prelude::col;
use hmw_parquet::datafusion::prelude::lit;
use imma_parser::traits::PositionExt;
use imma_parser::types::IMMARecord;
use imma_parser::types::NaiveTimeWrapper;

use super::predicate_projection::Epoch;
use super::predicate_projection::LatticeFilter;
use super::predicate_projection::MarineWeatherObservationPredicateProjection;
use super::predicate_projection::MonthFilter;
use super::predicate_projection::Project;
use super::predicate_projection::ToExpr;

#[derive(
    arrow_convert::ArrowField,
    arrow_convert::ArrowSerialize,
    arrow_convert::ArrowDeserialize,
    Debug,
    PartialEq,
)]
pub struct MarineWeatherObservation {
    pub year: Option<u16>,
    pub month: Option<u8>,
    pub day: Option<u8>,
    pub latticed_point: Option<LatticedPoint>,
    pub position: Option<imma_parser::types::Position>,
    pub time: Option<NaiveTimeWrapper>,
    pub time_indicator: Option<imma_parser::types::TimeIndicator>,
    pub ship: Option<imma_parser::types::Ship>,
    pub wind: Option<imma_parser::types::Wind>,
    pub visibility: Option<imma_parser::types::Visibility>,
    pub waves: Option<imma_parser::types::Waves>,
    pub swell: Option<imma_parser::types::Waves>,
    pub attm_uida: Option<imma_parser::attm_types::UidaAttachment>,
}

impl MarineWeatherObservation {
    const UID_COLUMN: &str = "attm_uida";
    const YEAR_COLUMN: &str = "year";
    const MONTH_COLUMN: &str = "month";
    const LATTICED_POINT_COLUMN: &str = "latticed_point";

    pub fn predicate<P: Project>(
        epoch: &Epoch,
        months: &MonthFilter,
        lattice_filter: &LatticeFilter,
    ) -> MarineWeatherObservationPredicateProjection {
        let epoch_filter = epoch.to_filter_expr(col(Self::YEAR_COLUMN));
        let month_filter = months.to_filter_expr(col(Self::MONTH_COLUMN));
        let latticed_point_filter = lattice_filter.to_filter_expr(col(Self::LATTICED_POINT_COLUMN));

        let filter = latticed_point_filter
            .and(month_filter.clone())
            .and(epoch_filter.clone())
            .and(P::filter());

        let projection = P::columns();
        MarineWeatherObservationPredicateProjection {
            columns: projection,
            filter,
            sort: Vec::with_capacity(0),
            distinct_on: col(Self::UID_COLUMN),
        }
    }

    pub fn project<P: Project>(self) -> P {
        P::project(self)
    }

    pub fn new_from_imma(mut imma: IMMARecord) -> Self {
        let position = imma.position_as_point();

        let latticed_point = position.and_then(|p| p.try_into().ok());
        Self {
            year: imma.date_time.as_ref().and_then(|d| d.raw_parts.year),
            month: imma.date_time.as_ref().and_then(|d| d.raw_parts.month),
            day: imma.date_time.as_ref().and_then(|d| d.raw_parts.day),
            latticed_point,
            position: imma.position.take(),
            time: imma
                .date_time
                .as_ref()
                .and_then(|dt| dt.time.clone())
                .map(|t| t.time),
            time_indicator: imma.date_time.and_then(|dt| dt.time).map(|t| t.indicator),
            ship: imma.ship,
            wind: imma.wind,
            visibility: imma.visibility,
            waves: imma.waves,
            swell: imma.swell,
            attm_uida: imma.attm_uida,
        }
    }
}

impl AsParquet for MarineWeatherObservation {
    type Partition = [u8; 2];
    type Predicate = MarineWeatherObservationPredicateProjection;

    const SORT_FIELDS: &[&str] = &["latticed_point", "year", "month", "day"];

    fn partition(&self) -> Self::Partition {
        let month = self.month.unwrap_or_default();
        let latticed_point = match &self.latticed_point {
            Some(d) => d[0],
            None => 0,
        };
        [month, latticed_point]
    }

    fn read(
        predicate_projection: &MarineWeatherObservationPredicateProjection,
        df: DataFrame,
    ) -> Result<DataFrame, DataFusionError> {
        let mut df = df.filter(predicate_projection.filter.clone())?;
        let mut columns = predicate_projection.columns.clone();
        if columns.is_empty() {
            columns = df
                .schema()
                .fields()
                .iter()
                .map(|field| col(field.name()))
                .collect::<Vec<_>>();
        }

        // Pushing the distinct on column here so we can de-duplicate later.
        if !columns.contains(&predicate_projection.distinct_on) {
            columns.push(predicate_projection.distinct_on.clone());
        }

        df = df.select(columns)?;
        if !predicate_projection.sort.is_empty() {
            df = df.sort(predicate_projection.sort.clone())?;
        }

        Ok(df)
    }

    fn underlying_type(&self) -> &Self::Type {
        self
    }
}

impl Project for MarineWeatherObservation {
    fn columns() -> Vec<Expr> {
        vec![]
    }

    fn project(data: MarineWeatherObservation) -> Self {
        data
    }

    fn filter() -> Expr {
        lit(true)
    }
}

macro_rules! extract_stat_from_batch {
    ($batch:expr, $stat:ident, $type_int:ty, $type_array: ty) => {{
        let count_array = $batch
            .column_by_name(stringify!($stat))
            .ok_or(DataFusionError::External("Column not found".into()))?
            .as_any()
            .downcast_ref::<$type_array>()
            .ok_or(DataFusionError::External("Column not correct type".into()))?;
        match count_array.is_null(0) {
            true => None,
            false => Some(
                <$type_int>::try_from(count_array.value(0))
                    .map_err(|_| DataFusionError::External("Count is negative".into()))?,
            ),
        }
    }};
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MarineWeatherObservationDataStats {
    pub min_year: u16,
    pub max_year: u16,
    pub overall_count: u64,
}

impl DataStats for MarineWeatherObservation {
    type Stats = MarineWeatherObservationDataStats;

    fn build(df: DataFrame) -> Result<DataFrame, DataFusionError> {
        df.aggregate(
            vec![],
            vec![
                min(col("year")).alias("min_year"),
                max(col("year")).alias("max_year"),
                count_all().alias("overall_count"),
            ],
        )
    }

    fn stats(batch: arrow::array::RecordBatch) -> Result<Option<Self::Stats>, DataFusionError> {
        let min_year = extract_stat_from_batch!(batch, min_year, u16, UInt16Array);
        let max_year = extract_stat_from_batch!(batch, max_year, u16, UInt16Array);
        let overall_count =
            extract_stat_from_batch!(batch, overall_count, u64, Int64Array).unwrap_or_default();
        let (Some(min_year), Some(max_year)) = (min_year, max_year) else {
            return Ok(None);
        };

        Ok(Some(MarineWeatherObservationDataStats {
            min_year,
            max_year,
            overall_count,
        }))
    }
}
