use std::path::PathBuf;

use clap::{
    Arg, ArgAction, Command, ValueHint,
    builder::{OsStr, PathBufValueParser, TypedValueParser},
    command,
};
use download_raw_imma::download_imma_files;
use hmw_data::WriterOptions;
use process_imma::ProcessInput;

mod download_raw_imma;
mod generate_lattice;
mod process_imma;

#[tokio::main]
async fn main() {
    let command = command!()
        .subcommand(build_download_command())
        .subcommand(build_generate_lattice_command())
        .subcommand(build_process_imma_data_command());
    let mut command_clone = command.clone();

    let matches = command.get_matches();
    if let Some(matches) = matches.subcommand_matches("icoads-fetch") {
        let output_dir = matches
            .get_one::<PathBuf>("destination")
            .expect("destination required");
        let start_year = matches
            .get_one::<i32>("start-year")
            .expect("start-year required");
        let end_year = matches
            .get_one::<i32>("end-year")
            .expect("end-year required");
        download_imma_files(*start_year, *end_year, output_dir.clone()).await;
    } else if let Some(matches) = matches.subcommand_matches("generate-lattice") {
        let mask = matches.get_one::<PathBuf>("mask").expect("mask required");
        let out = matches.get_one::<PathBuf>("out").expect("out required");
        generate_lattice::generate_haversine_lattice(mask, out).await;
    } else if let Some(matches) = matches.subcommand_matches("icoads-process") {
        let out = matches
            .get_one::<PathBuf>("destination")
            .expect("destination required")
            .to_path_buf();
        let input = match matches.get_many::<PathBuf>("source") {
            Some(sources) => ProcessInput::Local(sources.cloned().collect::<Vec<_>>()),
            None => ProcessInput::RemoteYears {
                start_year: *matches
                    .get_one::<i32>("start-year")
                    .expect("start-year required when source missing"),
                end_year: *matches
                    .get_one::<i32>("end-year")
                    .expect("end-year required when source missing"),
            },
        };
        let number_of_files = matches
            .get_one::<u8>("number-of-files")
            .expect("number-of-files required");
        let max_batch_size = matches
            .get_one::<u32>("max-batch-size")
            .expect("max-batch-size required");
        let max_in_flight = matches
            .get_one::<u32>("max-in-flight")
            .expect("max-in-flight required");
        let parquet_page_row_count_limit = matches
            .get_one::<u32>("parquet-page-row-count-limit")
            .expect("parquet-page-row-count-limit required");
        process_imma::process_imma(
            input,
            out,
            *number_of_files as usize,
            *max_batch_size as usize,
            *max_in_flight as usize,
            *parquet_page_row_count_limit as usize,
        )
        .await;
    } else {
        command_clone.print_help().expect("can print help");
    }
}

fn build_download_command() -> Command {
    Command::new("icoads-fetch")
        .alias("fetch")
        .about("Fetch ICOADS IMMA files from the NOAA remote indexes")
        .arg(
            Arg::new("destination")
                .short('d')
                .long("destination")
                .help("Output the files to this directory.")
                .required(true)
                .action(ArgAction::Set)
                .value_parser(PathBufValueParser::new())
                .value_hint(ValueHint::DirPath),
        )
        .arg(
            Arg::new("start-year")
                .long("start-year")
                .help("Start year to download, inclusive.")
                .required(true)
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(i32)),
        )
        .arg(
            Arg::new("end-year")
                .long("end-year")
                .help("End year to download, inclusive.")
                .required(true)
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(i32)),
        )
}

fn build_generate_lattice_command() -> Command {
    Command::new("generate-lattice")
        .alias("gen-lat")
        .about("Generate lattice ")
        .arg(
            Arg::new("mask")
                .short('m')
                .long("mask")
                .help("SHP file to read a lattice mask from.")
                .required(true)
                .action(ArgAction::Set)
                .value_parser(PathBufValueParser::new())
                .value_hint(ValueHint::DirPath),
        )
        .arg(
            Arg::new("out")
                .short('o')
                .long("out")
                .help("Json file to store the lattice in.")
                .required(true)
                .action(ArgAction::Set)
                .value_parser(PathBufValueParser::new())
                .value_hint(ValueHint::FilePath),
        )
}

fn build_process_imma_data_command() -> Command {
    let default_write_options = WriterOptions::default();
    Command::new("icoads-process")
        .about(
            "Process ICOADS IMMA data file and create parquet files that can be used for analysis.",
        )
        .arg(
            Arg::new("source")
                .short('s')
                .long("source")
                .help("Read IMMA files from this local file or directory.")
                .action(ArgAction::Append)
                .required_unless_present_all(["start-year", "end-year"])
                .conflicts_with_all(["start-year", "end-year"])
                .value_parser(PathBufValueParser::new())
                .value_hint(ValueHint::DirPath),
        )
        .arg(
            Arg::new("start-year")
                .long("start-year")
                .help("Start year to process from the remote NOAA index, inclusive.")
                .required_unless_present("source")
                .requires("end-year")
                .conflicts_with("source")
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(i32)),
        )
        .arg(
            Arg::new("end-year")
                .long("end-year")
                .help("End year to process from the remote NOAA index, inclusive.")
                .required_unless_present("source")
                .requires("start-year")
                .conflicts_with("source")
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(i32)),
        )
        .arg(
            Arg::new("destination")
                .short('d')
                .long("destination")
                .help("Output parquet files to this directory.")
                .required(true)
                .action(ArgAction::Set)
                .value_parser(PathBufValueParser::new().try_map(verify_dir))
                .value_hint(ValueHint::DirPath),
        )
        .arg(
            Arg::new("number-of-files")
                .short('n')
                .long("number-of-files")
                .help("Number of files to write to.")
                .default_value(OsStr::from(
                    default_write_options.number_of_files.to_string(),
                ))
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(u8).range(1..)),
        )
        .arg(
            Arg::new("max-batch-size")
                .short('b')
                .long("max-batch-size")
                .help("Max batch size. This is the number of rows in a row group.")
                .default_value(OsStr::from(
                    default_write_options.max_batch_size.to_string(),
                ))
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(u32).range(1..)),
        )
        .arg(
            Arg::new("max-in-flight")
                .short('i')
                .long("max-in-flight")
                .help("Max number of items to keep in flight.")
                .default_value(OsStr::from(default_write_options.max_in_flight.to_string()))
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(u32).range(1..)),
        )
        .arg(
            Arg::new("parquet-page-row-count-limit")
                .short('p')
                .long("parquet-page-row-count-limit")
                .help("Parquet page row count limit.")
                .default_value(OsStr::from(
                    default_write_options
                        .parquet_page_row_count_limit
                        .to_string(),
                ))
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(u32).range(1..)),
        )
}

fn verify_dir(path: PathBuf) -> Result<PathBuf, &'static str> {
    match path.try_exists() {
        Ok(true) => (),
        Ok(false) => return Err("Supplied directory not found."),
        Err(_) => return Err("Error looking up directory."),
    };
    if !path.is_dir() {
        return Err("Supplied location is not a directory.");
    }
    Ok(path)
}
