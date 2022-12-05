use crate::csv::CsvDumper;
use crate::filter::AccountFilter;

use clap::Parser;
use log::{error, info};
use reqwest::blocking::Response;
use solana_snapshot_etl::archived::ArchiveSnapshotExtractor;
use solana_snapshot_etl::{AppendVecIterator, SnapshotExtractor};
use std::fs::{File};
use std::path::{Path};

mod csv;
mod filter;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Fetch the account for the specified public key
    #[clap(short, long)]
    pubkey: Vec<String>,

    /// Fetch all the accounts owned by the specified program id
    #[clap(short, long)]
    owner: Vec<String>,

    #[clap(help = "Snapshot archive file")]
    source: String,
}

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
    if let Err(e) = _main() {
        error!("{}", e);
        std::process::exit(1);
    }
}

fn _main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let filter = AccountFilter::new(&args.pubkey, &args.owner)?;
    let mut loader = SupportedLoader::new(&args.source)?;

    info!("Dumping to CSV");
    let mut writer = CsvDumper::new(filter);
    for append_vec in loader.iter() {
        writer.dump_append_vec(append_vec?);
    }
    drop(writer);
    info!("Done!");

    Ok(())
}

pub enum SupportedLoader {
    ArchiveFile(ArchiveSnapshotExtractor<File>),
    ArchiveDownload(ArchiveSnapshotExtractor<Response>),
}

impl SupportedLoader {
    fn new(
        source: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if source.starts_with("http://") || source.starts_with("https://") {
            Self::new_download(source)
        } else {
            Self::new_file(source.as_ref()).map_err(Into::into)
        }
    }

    fn new_download(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let resp = reqwest::blocking::get(url)?;
        let loader = ArchiveSnapshotExtractor::from_reader(resp)?;
        info!("Streaming snapshot from HTTP");
        Ok(Self::ArchiveDownload(loader))
    }

    fn new_file(
        path: &Path,
    ) -> solana_snapshot_etl::Result<Self> {
        Ok(
            Self::ArchiveFile(ArchiveSnapshotExtractor::open(path)?)
        )
    }
}

impl SnapshotExtractor for SupportedLoader {
    fn iter(&mut self) -> AppendVecIterator<'_> {
        match self {
            SupportedLoader::ArchiveFile(loader) => Box::new(loader.iter()),
            SupportedLoader::ArchiveDownload(loader) => Box::new(loader.iter()),
        }
    }
}
