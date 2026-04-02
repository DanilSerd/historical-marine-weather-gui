use std::{collections::HashMap, ops::RangeInclusive, sync::LazyLock};

use reqwest::Client;
use scraper::{Html, Selector};
use url::Url;

use crate::Error;

const FINAL_INDEX_URL: &str = "https://www.ncei.noaa.gov/data/international-comprehensive-ocean-atmosphere/v3/archive/final-untrim/";
const NRT_INDEX_URL: &str = "https://www.ncei.noaa.gov/data/international-comprehensive-ocean-atmosphere/v3/archive/nrt/monthly/";

static ROW_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("table tr").expect("valid row selector"));
static CELL_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("td").expect("valid cell selector"));
static LINK_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("a").expect("valid link selector"));

/// A remote IMMA file discovered in an index page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteImmaFile {
    /// The file year.
    pub year: i32,
    /// The file month.
    pub month: u8,
    /// The full file URL.
    pub url: Url,
    /// The file size in bytes.
    pub size: u64,
    /// Whether the file came from the near-real-time listing.
    pub nrt: bool,
}

/// An index of remote IMMA files built from a final and a near-real-time listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFileIndex {
    files: Vec<RemoteImmaFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IndexedRemoteImmaFile {
    file: RemoteImmaFile,
    nrt_revision_date: Option<u32>,
}

impl RemoteFileIndex {
    /// Build a remote file index from the NOAA final and near-real-time listings.
    pub async fn from_noaa() -> Result<Self, Error> {
        Self::from_index_urls(
            Url::parse(FINAL_INDEX_URL).expect("valid final NOAA index URL"),
            Url::parse(NRT_INDEX_URL).expect("valid NRT NOAA index URL"),
        )
        .await
    }

    /// Build a remote file index from the provided final and near-real-time index URLs.
    pub async fn from_index_urls(final_index_url: Url, nrt_index_url: Url) -> Result<Self, Error> {
        let client = Client::new();
        let final_request_url = final_index_url.clone();
        let nrt_request_url = nrt_index_url.clone();

        let (final_page, nrt_page) = tokio::try_join!(
            async {
                client
                    .get(final_request_url)
                    .send()
                    .await?
                    .error_for_status()?
                    .text()
                    .await
                    .map_err(Error::from)
            },
            async {
                client
                    .get(nrt_request_url)
                    .send()
                    .await?
                    .error_for_status()?
                    .text()
                    .await
                    .map_err(Error::from)
            },
        )?;

        let mut indexed_files = parse_listing(&final_page, &final_index_url, false)?;
        indexed_files.extend(parse_listing(&nrt_page, &nrt_index_url, true)?);

        let files = indexed_files
            .into_iter()
            .try_fold(
                HashMap::<(i32, u8), IndexedRemoteImmaFile>::new(),
                |mut files_by_month, indexed_file| {
                    let key = (indexed_file.file.year, indexed_file.file.month);
                    match files_by_month.remove(&key) {
                        Some(existing_file) if existing_file.file.nrt && indexed_file.file.nrt => {
                            let file_to_keep = match (
                                existing_file.nrt_revision_date,
                                indexed_file.nrt_revision_date,
                            ) {
                                (Some(existing_date), Some(indexed_date))
                                    if indexed_date > existing_date =>
                                {
                                    indexed_file
                                }
                                _ => existing_file,
                            };
                            files_by_month.insert(key, file_to_keep);
                            Ok(files_by_month)
                        }
                        Some(existing_file) => Err(Error::RemoteFileIndexError(format!(
                            "overlapping remote files for {:04}-{:02}: {} and {}",
                            key.0, key.1, existing_file.file.url, indexed_file.file.url
                        ))),
                        None => {
                            files_by_month.insert(key, indexed_file);
                            Ok(files_by_month)
                        }
                    }
                },
            )?
            .into_values()
            .map(|indexed_file| indexed_file.file)
            .collect::<Vec<_>>();

        let mut files = files;
        files.sort_by(|left, right| {
            (left.year, left.month, left.nrt, left.url.as_str()).cmp(&(
                right.year,
                right.month,
                right.nrt,
                right.url.as_str(),
            ))
        });

        Ok(Self { files })
    }

    /// Iterate over files whose year falls within the provided inclusive range.
    pub fn iter_in_year_range(
        &self,
        years: RangeInclusive<i32>,
    ) -> impl Iterator<Item = &RemoteImmaFile> {
        self.files
            .iter()
            .filter(move |file| years.contains(&file.year))
    }
}

/// Parse a supported IMMA file name into `(year, month)`.
pub(crate) fn parse_year_month(file_name: &str) -> Option<(i32, u8)> {
    [false, true]
        .into_iter()
        .find_map(|nrt| parse_year_month_for_listing(file_name, nrt))
}

fn parse_year_month_for_listing(file_name: &str, nrt: bool) -> Option<(i32, u8)> {
    let year_month = match nrt {
        true => file_name
            .strip_prefix("icoads-nrt_")
            .filter(|_| file_name.contains("_final_") && file_name.ends_with(".dat.gz"))
            .and_then(|_| file_name.split("_d").nth(1))
            .and_then(|remainder| remainder.split_once('_').map(|(year_month, _)| year_month)),
        false => file_name
            .strip_prefix("IMMA1_")
            .filter(|_| file_name.ends_with(".gz"))
            .and_then(|_| file_name.strip_suffix(".gz"))
            .and_then(|trimmed| trimmed.rsplit_once('_').map(|(_, year_month)| year_month)),
    }?;

    let (year, month) = match nrt {
        true if year_month.len() == 6 => (&year_month[..4], &year_month[4..]),
        false => year_month.split_once('-')?,
        _ => return None,
    };

    let year = year.parse().ok()?;
    let month = month.parse::<u8>().ok()?;
    (1..=12).contains(&month).then_some((year, month))
}

fn parse_listing(
    page: &str,
    base_url: &Url,
    nrt: bool,
) -> Result<Vec<IndexedRemoteImmaFile>, Error> {
    Html::parse_document(page)
        .select(&ROW_SELECTOR)
        .filter_map(|row| {
            let cells = row.select(&CELL_SELECTOR).collect::<Vec<_>>();
            let name_cell = cells.first()?;
            let size_cell = cells.get(2)?;
            let link = name_cell.select(&LINK_SELECTOR).next()?;
            let href = link.value().attr("href")?;
            let file_name = href.rsplit('/').next().unwrap_or(href);
            parse_year_month_for_listing(file_name, nrt).map(|(year, month)| {
                let nrt_revision_date = match nrt {
                    true => file_name
                        .split("_c")
                        .nth(1)
                        .and_then(|remainder| remainder.split_once('.').map(|(date, _)| date))
                        .ok_or_else(|| {
                            Error::RemoteListingParseError(format!(
                                "missing NRT revision date for {file_name}"
                            ))
                        })?
                        .parse::<u32>()
                        .map(Some)
                        .map_err(|error| {
                            Error::RemoteListingParseError(format!(
                                "failed to parse NRT revision date for {file_name}: {error}"
                            ))
                        })?,
                    false => None,
                };
                let size = size_cell
                    .text()
                    .collect::<String>()
                    .trim()
                    .parse::<u64>()
                    .map_err(|error| {
                        Error::RemoteListingParseError(format!(
                            "failed to parse size for {file_name}: {error}"
                        ))
                    })?;
                let url = base_url.join(href).map_err(|error| {
                    Error::RemoteListingParseError(format!(
                        "failed to build URL for {file_name}: {error}"
                    ))
                })?;

                Ok(IndexedRemoteImmaFile {
                    file: RemoteImmaFile {
                        year,
                        month,
                        url,
                        size,
                        nrt,
                    },
                    nrt_revision_date,
                })
            })
        })
        .collect::<Result<Vec<_>, Error>>()
}
