use std::collections::HashMap;

use async_stream::try_stream;
use chrono::NaiveTime;
use futures::pin_mut;
use hmw_data::{
    BeaufortScaleBucket, BeaufortScaleBucketer, CardinalOrdinalDirection,
    DirectionalBucketingError, DirectionalIntensity, DirectionalIntensityHistogram,
    WindObservation,
};
use hmw_geo::geo::point;
use imma_parser::types::WindDir;

#[tokio::test]
async fn test_directional_histogram_with_beaufort_wind_observation() {
    let stream = try_stream! {
        /// Considered:
        yield WindObservation { latticed_point: point!(x: 1.0, y: 1.0).try_into().ok(), year: Some(1990), month: Some(1), day: Some(1), time: NaiveTime::from_hms_opt(0, 0, 0), wind_direction: Some(WindDir::Direction(45)), wind_speed: Some(10.0) };
        yield WindObservation { latticed_point: point!(x: 2.0, y: 1.0).try_into().ok(), year: Some(1991), month: Some(1), day: Some(1), time: NaiveTime::from_hms_opt(1, 0, 0), wind_direction: Some(WindDir::Calm), wind_speed: None };
        yield WindObservation { latticed_point: point!(x: 3.0, y: 1.0).try_into().ok(), year: Some(1991), month: Some(1), day: None, time: NaiveTime::from_hms_opt(2, 0, 0), wind_direction: Some(WindDir::Variable), wind_speed: Some(0.2)  };
        yield WindObservation { latticed_point: point!(x: 4.0, y: 1.0).try_into().ok(), year: Some(1992), month: Some(1), day: Some(1), time: None, wind_direction: Some(WindDir::Calm), wind_speed: Some(0.2) };
        yield WindObservation { latticed_point: point!(x: 5.0, y: 1.0).try_into().ok(), year: Some(1992), month: Some(1), day: Some(2), time: NaiveTime::from_hms_opt(4, 0, 0), wind_direction: Some(WindDir::Direction(180)), wind_speed: Some(0.2) };
        /// Ignored:
        yield WindObservation { latticed_point: point!(x: 6.0, y: 1.0).try_into().ok(), year: Some(1990), month: Some(1), day: Some(1), time: NaiveTime::from_hms_opt(5, 0, 0), wind_direction: Some(WindDir::Direction(0)), wind_speed: None };
        yield WindObservation { latticed_point: point!(x: 7.0, y: 1.0).try_into().ok(), year: Some(1990), month: Some(1), day: Some(1), time: NaiveTime::from_hms_opt(6, 0, 0), wind_direction: Some(WindDir::Calm), wind_speed: Some(10.0) };
        yield WindObservation { latticed_point: point!(x: 8.0, y: 1.0).try_into().ok(), year: Some(1990), month: Some(1), day: Some(1), time: NaiveTime::from_hms_opt(7, 0, 0), wind_direction: Some(WindDir::Variable), wind_speed: Some(10.0) };
    };

    pin_mut!(stream);

    let histogram: DirectionalIntensityHistogram<BeaufortScaleBucketer> =
        DirectionalIntensityHistogram::populate(stream)
            .await
            .unwrap();

    let bins = histogram.iter_non_empty().collect::<Vec<_>>();

    assert_eq!(bins.len(), 2);
    assert!(bins.contains(&DirectionalIntensity {
        intensity_bucket: BeaufortScaleBucket::Calm,
        direction_bucket: CardinalOrdinalDirection::Indeterminate,
        probability: 0.8,
        count: 4
    }));
    assert!(bins.contains(&DirectionalIntensity {
        intensity_bucket: BeaufortScaleBucket::FreshBreeze,
        direction_bucket: CardinalOrdinalDirection::NE,
        probability: 0.2,
        count: 1
    }));

    let stats = histogram.stats();

    assert_eq!(stats.histogram_counters.inserted, 5);

    assert_eq!(
        stats.histogram_counters.skipped,
        HashMap::from([
            (DirectionalBucketingError::UnknownIntensity, 1),
            (DirectionalBucketingError::Inconsistent, 1),
            (DirectionalBucketingError::UnknownDirection, 1),
        ])
    );

    assert_eq!(stats.date_time.counters.missing_date, 1);
    assert_eq!(stats.date_time.counters.missing_time, 1);
    assert_eq!(
        stats
            .date_time
            .iter_year(1990..=1992)
            .map(|bucket| (bucket.year, bucket.count))
            .collect::<Vec<_>>(),
        vec![(1990, 1), (1991, 2), (1992, 2)]
    );
    assert_eq!(
        stats
            .date_time
            .iter_doy()
            .filter(|bucket| bucket.count > 0)
            .map(|bucket| (bucket.day, bucket.count))
            .collect::<Vec<_>>(),
        vec![(0, 3), (1, 1)]
    );
    assert_eq!(
        stats
            .date_time
            .iter_hod()
            .filter(|bucket| bucket.count > 0)
            .map(|bucket| (bucket.hour, bucket.count))
            .collect::<Vec<_>>(),
        vec![(0, 1), (1, 1), (2, 1), (4, 1)]
    );
}
