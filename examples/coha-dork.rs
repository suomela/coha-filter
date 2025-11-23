use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use coha_filter::{Coha, CohaSearch};
use log::info;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Directory where the COHA corpus is located
    corpus_dir: PathBuf,
    /// Where to store results
    result_dir: PathBuf,
    #[command(flatten)]
    verbose: Verbosity<InfoLevel>,
}

fn run(args: &Args) -> Result<()> {
    let coha = Coha::load(&args.corpus_dir)?;

    let f_dork = coha.get_filter(|w| w.lemma == "dork");

    let s_dork = CohaSearch {
        label: "dork".to_owned(),
        filter_list: vec![&f_dork],
    };
    coha.search(&args.result_dir, &[&s_dork])?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .init();
    run(&args)?;
    info!("all done");
    Ok(())
}
