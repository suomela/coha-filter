use coha_filter::{cmdline_err, Coha, CohaFilter, CohaSearch, MyError};
use log::{error, info};
use regex::Regex;
use std::env;
use std::path::PathBuf;
use std::process;

struct Settings {
    work_dir: PathBuf,
    result_dir: PathBuf,
}

fn get_args() -> Result<Settings, MyError> {
    let mut args = env::args();
    args.next();
    let mut get_path_arg = |what| {
        let a = args.next();
        match a {
            None => Err(cmdline_err(&format!(
                "command line argument {what} missing"
            ))),
            Some(s) => Ok(PathBuf::from(s)),
        }
    };
    let work_dir = get_path_arg("WORK_DIR")?;
    let result_dir = get_path_arg("RESULT_DIR")?;
    Ok(Settings {
        work_dir,
        result_dir,
    })
}

fn run() -> Result<(), MyError> {
    let settings = get_args()?;
    let coha = Coha::load(&settings.work_dir)?;

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
        &settings.result_dir,
        &[&s_be_going_to_verb, &s_gonna_verb, &s_gonna_any],
    )?;
    Ok(())
}

fn main() {
    env_logger::init();
    if let Err(e) = run() {
        error!("{e}");
        process::exit(1);
    }
    info!("all done");
}
