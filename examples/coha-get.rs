use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use coha_filter::{Coha, CohaSearch};
use log::info;
use regex::Regex;
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

    let re_v_n = Regex::new(r"^v.n").unwrap();
    let f_v_n = coha.get_filter(|w| re_v_n.is_match(&w.pos));
    let f_get = coha.get_filter(|w| w.word == "get");

    let s_get = CohaSearch {
        label: "get".to_owned(),
        filter_list: vec![&f_get, &f_v_n],
    };
    coha.search(&args.result_dir, &[&s_get])?;
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
