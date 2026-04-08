use std::{fmt::Display, ops::RangeInclusive};

use hmw_geo::LatticeEntry;
use hmw_parquet::datafusion::{
    logical_expr::SortExpr,
    prelude::{Expr, lit},
};
use serde::{Deserialize, Serialize};

use super::weather_observation::MarineWeatherObservation;

pub struct MarineWeatherObservationPredicateProjection {
    /// Columns to select.
    pub columns: Vec<Expr>,
    /// Filter.
    pub filter: Expr,
    /// Sort by.
    pub sort: Vec<SortExpr>,
    /// Distinct on expr. Not applied at datafusion layer.
    pub distinct_on: Expr,
}

#[derive(Debug, Clone)]
pub enum MonthFilter {
    All,
    Unknown,
    Months(Vec<chrono::Month>),
}

impl ToExpr for MonthFilter {
    fn to_filter_expr(&self, col: Expr) -> Expr {
        match self {
            MonthFilter::All => col.is_not_null(),
            MonthFilter::Unknown => col.is_null(),
            MonthFilter::Months(months) => col.in_list(
                months
                    .iter()
                    .map(|m| lit(m.number_from_month() as u8))
                    .collect(),
                false,
            ),
        }
    }
}

#[derive(Debug)]
pub enum LatticeFilter {
    /// Some lattice entries.
    Lattice(Vec<LatticeEntry>),
    /// Null lattice entry.
    Unknown,
}

impl ToExpr for LatticeFilter {
    fn to_filter_expr(&self, col: Expr) -> Expr {
        let mut predicates = match self {
            LatticeFilter::Lattice(n) => n
                .iter()
                .map(|entry| {
                    let pattern = format!("{}%", AsRef::<str>::as_ref(entry));
                    col.clone().like(lit(pattern))
                })
                .collect::<Vec<_>>(),
            LatticeFilter::Unknown => return col.is_null(),
        };

        // Build balanced or tree to minimize the chance of blowing up the stack.
        while predicates.len() > 1 {
            predicates = predicates
                .chunks(2)
                .map(|chunk| match chunk {
                    [left, right] => left.clone().or(right.clone()),
                    [expr] => expr.clone(),
                    _ => unreachable!(),
                })
                .collect();
        }

        predicates.pop().unwrap_or_else(|| lit(false))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Epoch {
    Range(RangeInclusive<u16>),
    Unknown,
}

impl Epoch {
    pub fn get_year_range(&self) -> Option<RangeInclusive<u16>> {
        let dr = match self {
            Epoch::Range(range) => range.clone(),
            Epoch::Unknown => return None,
        };
        Some(dr)
    }
}

impl Display for Epoch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Epoch::Range(range) => {
                if range.start() == range.end() {
                    write!(f, "{}", range.start())
                } else {
                    write!(f, "{} - {}", range.start(), range.end())
                }
            }
            Epoch::Unknown => write!(f, "Unknown"),
        }
    }
}

impl ToExpr for Epoch {
    fn to_filter_expr(&self, col: Expr) -> Expr {
        match self {
            Epoch::Unknown => col.is_null(),
            Epoch::Range(range) => {
                let start = lit(*range.start());
                let end = lit(*range.end());
                col.clone().gt_eq(start).and(col.lt_eq(end))
            }
        }
    }
}

pub trait ToExpr {
    /// Returns [`Expr`] where the col is filtered by a predicate.
    fn to_filter_expr(&self, col: Expr) -> Expr;
}

/// Main projection trait.
pub trait Project: Sized {
    /// Which columns to select for this projection.
    fn columns() -> Vec<Expr>;
    /// Return the projected type.
    fn project(data: MarineWeatherObservation) -> Self;
    /// Extra filtering to apply.
    fn filter() -> Expr;
}
