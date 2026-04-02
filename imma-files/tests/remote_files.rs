use std::{collections::HashSet, fs};

use axum::{Router, extract::State, response::Html, routing::get};
use imma_files::RemoteFileIndex;
use tokio::{net::TcpListener, task::JoinHandle};
use url::Url;

#[derive(Clone)]
struct FixtureState {
    final_fixture: String,
    nrt_fixture: String,
}

struct FixtureServer {
    base_url: Url,
    handle: JoinHandle<()>,
}

async fn spawn_fixture_server() -> FixtureServer {
    let app = Router::new()
        .route(
            "/final/",
            get(|State(state): State<FixtureState>| async move { Html(state.final_fixture) }),
        )
        .route(
            "/nrt/",
            get(|State(state): State<FixtureState>| async move { Html(state.nrt_fixture) }),
        )
        .with_state(FixtureState {
            final_fixture: read_fixture("final-untrim.html"),
            nrt_fixture: read_fixture("nrt-monthly.html"),
        });

    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let base_url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    FixtureServer { base_url, handle }
}

impl FixtureServer {
    fn final_index_url(&self) -> Url {
        self.base_url.join("final/").unwrap()
    }

    fn nrt_index_url(&self) -> Url {
        self.base_url.join("nrt/").unwrap()
    }
}

impl Drop for FixtureServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

fn read_fixture(file_name: &str) -> String {
    fs::read_to_string(format!(
        "{}/tests/data/{file_name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[tokio::test]
async fn builds_remote_file_index_from_local_indexes() {
    let server = spawn_fixture_server().await;
    let index = RemoteFileIndex::from_index_urls(server.final_index_url(), server.nrt_index_url())
        .await
        .unwrap();
    let files = index.iter_in_year_range(0..=3000).collect::<Vec<_>>();

    assert!(!files.is_empty());
    assert!(files.iter().all(|file| !file.url.path().ends_with(".pdf")));
    assert!(files.iter().all(|file| !file.url.path().ends_with(".nc")));
    assert!(
        files
            .iter()
            .all(|file| !file.url.path().contains("lastuid"))
    );
    assert!(
        files
            .iter()
            .all(|file| !file.url.path().contains("dupinventory"))
    );

    let final_file = files
        .iter()
        .copied()
        .find(|file| file.url.path().ends_with("IMMA1_R3.1.0_1662-12.gz"))
        .unwrap();
    assert_eq!(final_file.year, 1662);
    assert_eq!(final_file.month, 12);
    assert_eq!(final_file.size, 2512);
    assert!(!final_file.nrt);

    let nrt_file = files
        .iter()
        .copied()
        .find(|file| {
            file.url
                .path()
                .ends_with("icoads-nrt_r3.0.3_final_d202510_c20251116.dat.gz")
        })
        .unwrap();
    assert_eq!(nrt_file.year, 2025);
    assert_eq!(nrt_file.month, 10);
    assert_eq!(nrt_file.size, 390468687);
    assert!(nrt_file.nrt);

    let march_2024_files = files
        .iter()
        .copied()
        .filter(|file| file.year == 2024 && file.month == 3)
        .collect::<Vec<_>>();
    assert_eq!(march_2024_files.len(), 1);
    assert!(
        march_2024_files[0]
            .url
            .path()
            .ends_with("icoads-nrt_r3.0.2_final_d202403_c20240428.dat.gz")
    );
    assert!(files.iter().all(|file| {
        !file
            .url
            .path()
            .ends_with("icoads-nrt_r3.0.2_final_d202403_c20240425.dat.gz")
    }));

    let distinct_year_months = files
        .iter()
        .map(|file| (file.year, file.month))
        .collect::<HashSet<_>>();
    assert_eq!(distinct_year_months.len(), files.len());
}

#[tokio::test]
async fn iterates_files_in_year_range() {
    let server = spawn_fixture_server().await;
    let index = RemoteFileIndex::from_index_urls(server.final_index_url(), server.nrt_index_url())
        .await
        .unwrap();

    let early_files = index.iter_in_year_range(1662..=1663).collect::<Vec<_>>();
    assert!(!early_files.is_empty());
    assert!(
        early_files
            .iter()
            .all(|file| (1662..=1663).contains(&file.year))
    );
    assert!(early_files.iter().all(|file| !file.nrt));
    assert!(
        early_files
            .iter()
            .any(|file| file.url.path().ends_with("IMMA1_R3.1.0_1662-12.gz"))
    );

    let recent_files = index.iter_in_year_range(2025..=2025).collect::<Vec<_>>();
    assert!(!recent_files.is_empty());
    assert!(recent_files.iter().all(|file| file.year == 2025));
    assert!(recent_files.iter().all(|file| file.nrt));
    assert!(recent_files.iter().any(|file| {
        file.url
            .path()
            .ends_with("icoads-nrt_r3.0.3_final_d202510_c20251116.dat.gz")
    }));
}
