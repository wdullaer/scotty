use clap::{App, AppSettings, Arg, SubCommand};
use exitfailure::ExitFailure;
use failure::Error;
use std::convert::TryFrom;
use std::path::PathBuf;

use crate::index::{Index, IndexError};
use crate::init::Shell;

mod config;
mod index;
mod init;

fn main() -> Result<(), ExitFailure> {
    pretty_env_logger::init();
    let path_arg = Arg::with_name("path")
        .value_name("PATH")
        .help("The path to add into the index")
        .required(true);

    let target_arg = Arg::with_name("target")
        .value_name("TARGET")
        .help("The target to jump to")
        .required(true);

    let shell_arg = Arg::with_name("shell")
        .value_name("SHELL")
        .help("The shell scotty needs to integrate with")
        .required(true);

    let matches = App::new("scotty")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about("Transports you into a directory based on previous usage")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("add")
                .about("Add a path to the index")
                .arg(&path_arg),
        )
        .subcommand(
            SubCommand::with_name("search")
                .about("Searches a directory based on the input and the current index")
                .arg(&target_arg),
        )
        .subcommand(
            SubCommand::with_name("init")
                .about("Initegrates scotty in your shell")
                .arg(&shell_arg),
        )
        .get_matches();

    match matches.subcommand() {
        ("add", Some(sub_m)) => {
            let path = sub_m.value_of("path").expect("Path is missing");

            // TODO: investigate the log crate for error handling
            Ok(run_add(path)?)
        }
        ("search", Some(sub_m)) => {
            let target = sub_m.value_of("target").expect("Target is missing");

            Ok(run_search(target)?)
        }
        ("init", Some(sub_m)) => {
            let shell = sub_m.value_of("shell").expect("Shell is missing");

            Ok(run_init(shell)?)
        }
        _ => Ok(()),
    }
}

fn run_add(path: &str) -> Result<(), Error> {
    log::debug!("Running add with path: {}", path);
    let index = Index::open(config::get_index_config()?)?;
    let path_buf = PathBuf::from(path);
    index.add(&path_buf)?;
    Ok(())
}

fn run_search(target: &str) -> Result<(), Error> {
    log::debug!("Running search with target: {}", target);
    let index = Index::open(config::get_index_config()?)?;

    loop {
        let directory = match index.search(target)? {
            None => {
                return Err(Error::from(IndexError::NoResultsError {
                    pattern: target.to_owned(),
                }))
            }
            Some(d) => d,
        };
        if !directory.is_dir() {
            index.delete(&directory)?;
        } else {
            println!("{}", directory.display());
            break;
        }
    }
    Ok(())
}

fn run_init(target: &str) -> Result<(), Error> {
    log::debug!("Running init with shell: {}", target);
    let shell = Shell::try_from(target)?;
    Ok(init::init_shell(shell)?)
}
