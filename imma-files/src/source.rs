use std::{borrow::Cow, cmp::Ordering, path::PathBuf};

use url::Url;

use crate::remote_files::parse_year_month;

/// A local file path or a remote file URL.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileSource {
    /// A local file or directory path.
    Local(PathBuf),
    /// A remote file URL.
    Remote(Url),
}

impl PartialOrd for FileSource {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileSource {
    fn cmp(&self, other: &Self) -> Ordering {
        match (
            self.month_year_ordering_key(),
            other.month_year_ordering_key(),
        ) {
            (Some(left), Some(right)) => left
                .cmp(&right)
                .then_with(|| self.raw_ordering_key().cmp(&other.raw_ordering_key())),
            _ => self.raw_ordering_key().cmp(&other.raw_ordering_key()),
        }
    }
}

impl std::fmt::Display for FileSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(path) => write!(f, "{}", path.display()),
            Self::Remote(url) => write!(f, "{url}"),
        }
    }
}

impl FileSource {
    fn raw_ordering_key(&self) -> Cow<'_, str> {
        match self {
            Self::Local(path) => path.to_string_lossy(),
            Self::Remote(url) => Cow::Borrowed(url.as_str()),
        }
    }

    fn month_year_ordering_key(&self) -> Option<(u8, i32)> {
        self.file_name()
            .and_then(parse_year_month)
            .map(|(year, month)| (month, year))
    }

    fn file_name(&self) -> Option<&str> {
        match self {
            Self::Local(path) => path.file_name().and_then(|file_name| file_name.to_str()),
            Self::Remote(url) => url
                .path_segments()
                .and_then(|mut segments| segments.next_back()),
        }
    }

    pub(crate) fn extension(&self) -> Option<&str> {
        match self {
            Self::Local(path) => path.extension().and_then(|extension| extension.to_str()),
            Self::Remote(url) => url
                .path_segments()
                .and_then(|mut segments| segments.next_back())
                .and_then(|segment| segment.rsplit_once('.').map(|(_, extension)| extension))
                .filter(|extension| !extension.is_empty()),
        }
    }

    pub(crate) fn missing_local_path(&self) -> Option<Self> {
        match self {
            Self::Local(path) if !path.try_exists().unwrap_or(false) => {
                Some(Self::Local(path.clone()))
            }
            Self::Local(_) | Self::Remote(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_source(file_name: &str) -> FileSource {
        FileSource::Local(PathBuf::from(file_name))
    }

    fn remote_source(file_name: &str) -> FileSource {
        FileSource::Remote(Url::parse(&format!("https://example.com/{file_name}")).unwrap())
    }

    #[test]
    fn sorts_parseable_sources_by_month_then_year() {
        let mut sources = vec![
            local_source("IMMA1_R3.1.0_1900-12.gz"),
            local_source("IMMA1_R3.1.0_1901-01.gz"),
            local_source("IMMA1_R3.1.0_1900-01.gz"),
            local_source("IMMA1_R3.1.0_1899-02.gz"),
        ];

        sources.sort();

        assert_eq!(
            sources,
            vec![
                local_source("IMMA1_R3.1.0_1900-01.gz"),
                local_source("IMMA1_R3.1.0_1901-01.gz"),
                local_source("IMMA1_R3.1.0_1899-02.gz"),
                local_source("IMMA1_R3.1.0_1900-12.gz"),
            ]
        );
    }

    #[test]
    fn sorts_nrt_sources_by_month_then_year() {
        let mut sources = vec![
            remote_source("icoads-nrt_r3.0.3_final_d202510_c20251116.dat.gz"),
            remote_source("icoads-nrt_r3.0.2_final_d202403_c20240428.dat.gz"),
            remote_source("icoads-nrt_r3.0.2_final_d202401_c20240228.dat.gz"),
        ];

        sources.sort();

        assert_eq!(
            sources,
            vec![
                remote_source("icoads-nrt_r3.0.2_final_d202401_c20240228.dat.gz"),
                remote_source("icoads-nrt_r3.0.2_final_d202403_c20240428.dat.gz"),
                remote_source("icoads-nrt_r3.0.3_final_d202510_c20251116.dat.gz"),
            ]
        );
    }

    #[test]
    fn falls_back_to_raw_order_for_unparseable_sources() {
        let mut sources = vec![
            remote_source("icoads-nrt_r3.0.2_final_d202401_c20240228.dat.gz"),
            local_source("aaa-not-imma.gz"),
        ];

        sources.sort();

        assert_eq!(
            sources,
            vec![
                local_source("aaa-not-imma.gz"),
                remote_source("icoads-nrt_r3.0.2_final_d202401_c20240228.dat.gz"),
            ]
        );
    }
}
