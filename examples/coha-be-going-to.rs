use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use coha_filter::{Coha, CohaFilter, CohaSearch};
use log::info;
use regex::Regex;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Directory where the corpus is located
    work_dir: PathBuf,
    /// Where to store results
    result_dir: PathBuf,
    #[command(flatten)]
    verbose: Verbosity<InfoLevel>,
}

fn run(args: &Args) -> Result<()> {
    let coha = Coha::load(&args.work_dir)?;

    let re_vb = Regex::new(r"^vb").unwrap();
    let re_v_i = Regex::new(r"^v.i").unwrap();

    let f_vb = coha.get_filter(|w| re_vb.is_match(&w.pos));
    let f_v_i = coha.get_filter(|w| re_v_i.is_match(&w.pos));
    let f_going = coha.get_filter(|w| w.word == "going");
    let f_to = coha.get_filter(|w| w.word == "to");
    let f_gon = coha.get_filter(|w| w.word == "gon");
    let f_na = coha.get_filter(|w| w.word == "na");

    let s_be_going_to_verb = CohaSearch {
        label: "be-going-to-verb".to_owned(),
        filter_list: vec![&f_vb, &f_going, &f_to, &f_v_i],
    };
    let s_gonna_verb = CohaSearch {
        label: "gonna-verb".to_owned(),
        filter_list: vec![&f_gon, &f_na, &f_v_i],
    };
    let s_gonna_any = CohaSearch {
        label: "gonna-any".to_owned(),
        filter_list: vec![&f_gon, &f_na, &CohaFilter::Any],
    };
    coha.search(
        &args.result_dir,
        &[&s_be_going_to_verb, &s_gonna_verb, &s_gonna_any],
    )?;
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
