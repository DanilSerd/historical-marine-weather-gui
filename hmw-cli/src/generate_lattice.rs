use std::path::Path;

use colored::Colorize;
use hmw_geo::lattice_with_shp_file_mask;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub async fn generate_haversine_lattice(
    mask_file: impl AsRef<Path>,
    output_file: impl AsRef<Path>,
) {
    let path = mask_file.as_ref().to_path_buf();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut main_progress = None;
    let multi_progress = MultiProgress::new();

    let output_dir = output_file.as_ref().parent().unwrap_or(Path::new("."));

    if !output_dir.exists() {
        multi_progress
            .println(format!("Directory doesn't exist {}", output_dir.display()))
            .unwrap();
        return;
    }

    let handle = tokio::task::spawn_blocking(move || lattice_with_shp_file_mask(path, tx));

    while let Some(m) = rx.recv().await {
        match m {
            hmw_geo::LatticeBuildProgress::Start(ml) => {
                let p = start_generate_lattice_progress_bar("Lattice".into(), ml as u64);
                main_progress = Some(multi_progress.add(p));
            }
            hmw_geo::LatticeBuildProgress::Checked => {
                if let Some(p) = main_progress.as_mut() {
                    p.inc(1)
                }
            }
        }
    }

    match handle.await {
        Ok(Ok(l)) => {
            let stats = l.stats();
            multi_progress.println("").unwrap();
            multi_progress.println(stats.to_string()).unwrap();
            multi_progress.println("").unwrap();

            match l.to_file(&output_file) {
                Ok(_) => multi_progress
                    .println(format!(
                        "Lattice saved to {}",
                        output_file.as_ref().to_str().unwrap().blue()
                    ))
                    .unwrap(),
                Err(e) => multi_progress
                    .println(format!(
                        "Problem saving lattice to file: {}",
                        e.to_string().red()
                    ))
                    .unwrap(),
            }
        }
        Ok(Err(e)) => multi_progress
            .println(format!(
                "Problem generating lattice: {}",
                e.to_string().red()
            ))
            .unwrap(),
        Err(e) => multi_progress
            .println(format!(
                "Problem generating lattice: {}",
                e.to_string().red()
            ))
            .unwrap(),
    };
}

fn start_generate_lattice_progress_bar(prefix: String, len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    let pb = pb.with_message(prefix);
    pb.set_style(
        ProgressStyle::with_template("{msg}: {eta} {bar:40.green/green} {pos}/{len}")
            .unwrap()
            .progress_chars("##-"),
    );
    pb
}
