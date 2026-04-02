use chrono::Month;
use futures::{TryStreamExt, pin_mut};
use hmw_data::{
    DataReader, Epoch, FileSource, LatticeFilter, MarineWeatherObservation, MonthFilter,
    WriterOptions, process_imma_data_to_parquet,
};
use hmw_parquet::{DataVersion, ParquetWriter};

#[tokio::test]
async fn test_writer_and_reader() {
    let temp_dir = tempfile::TempDir::with_prefix("test-data").unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<hmw_data::Progress>();
    let mut input_dir = std::env::current_dir().unwrap();
    input_dir.push("tests/sample_imma_data");

    let writer = ParquetWriter::new(
        temp_dir.path(),
        WriterOptions {
            number_of_files: 1,
            max_batch_size: 50000,
            parquet_page_row_count_limit: 500,
            ..Default::default()
        },
        DataVersion::default(),
    )
    .unwrap();

    let files =
        process_imma_data_to_parquet(&[FileSource::Local(input_dir.clone())], &writer, 2, tx)
            .await
            .unwrap();

    files.commit().await.unwrap();

    let mut overall_items = 0;
    while let Some(p) = rx.recv().await {
        match p {
            hmw_data::Progress::Error((path, e)) => panic!("Error: {:?}, {}", path, e),
            hmw_data::Progress::ConversionComplete(items) => overall_items = items,
            _ => (),
        }
    }

    assert_eq!(overall_items, 648227);

    let reader = DataReader::new(temp_dir.path(), DataVersion::default(), None).unwrap();
    let mut explain = String::new();
    let lattice_filt = LatticeFilter::Lattice(vec!["ey".try_into().unwrap()]);
    let epoch_filt = Epoch::Range(1969..=1969);
    let month_filt = MonthFilter::Months(vec![Month::January]);
    let r = {
        let stream = reader
            .read::<MarineWeatherObservation>(
                &lattice_filt,
                &epoch_filt,
                &month_filt,
                Some((true, true, &mut explain)),
            )
            .await
            .unwrap();
        pin_mut!(stream);

        stream.try_collect::<Vec<_>>().await.unwrap()
    };
    dbg!(explain);
    assert!(r.iter().all(|o| o.latticed_point.is_some()
        && &AsRef::<[u8; 8]>::as_ref(&o.latticed_point.unwrap())[0..2] == b"ey"));
    assert!(r.iter().all(|o| o.month.is_some() && o.month.unwrap() == 1));
    assert!(r.iter().all(|o| {
        let position = o.position.as_ref().unwrap();
        position.lo <= 0. && position.lo >= -11.25
    }));
    assert_eq!(r.len(), 2478);

    let stats = reader.data_stats().await;
    let stats = stats.unwrap().unwrap();
    assert_eq!(stats.min_year, 1969,);
    assert_eq!(stats.max_year, 1969,);
    assert_eq!(stats.overall_count, overall_items as u64);
}
