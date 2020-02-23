// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use clap::{App, AppSettings, Arg, SubCommand};
use exitfailure::ExitFailure;
use failure::Error;
use std::convert::TryFrom;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::index::{Index, IndexError};
use crate::init::Shell;

mod config;
mod index;
mod init;
mod printer;

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

    let exclude_arg = Arg::with_name("exclude")
        .value_name("PATH")
        .long("exclude")
        .short("e")
        .number_of_values(1)
        .help("Exclude the given path from the search results");

    let all_arg = Arg::with_name("all")
        .long("all")
        .short("a")
        .help("Return all matched entries instead of only the most relevant one");

    let shell_arg = Arg::with_name("shell")
        .value_name("SHELL")
        .help("The shell scotty needs to integrate with")
        .required(true);

    let json_arg = Arg::with_name("json")
        .long("json")
        .help("Print output as a series of newline delimited json objects");

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
                .arg(&exclude_arg)
                .arg(&all_arg)
                .arg(&target_arg),
        )
        .subcommand(
            SubCommand::with_name("init")
                .about("Integrates scotty in your shell")
                .arg(&shell_arg),
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("Print the current index")
                .arg(&json_arg),
        )
        .get_matches();

    match matches.subcommand() {
        ("add", Some(sub_m)) => {
            let path = sub_m.value_of_os("path").expect("Path is missing");

            Ok(run_add(path)?)
        }
        ("search", Some(sub_m)) => {
            let target = sub_m.value_of("target").expect("Target is missing");
            let excluded_path = sub_m.value_of_os("exclude").map(|value| Path::new(value));
            let find_all = sub_m.is_present("all");

            Ok(run_search(target, excluded_path, find_all)?)
        }
        ("init", Some(sub_m)) => {
            let shell = sub_m.value_of_os("shell").expect("Shell is missing");

            Ok(run_init(shell)?)
        }
        ("list", Some(sub_m)) => {
            let is_json = sub_m.is_present("json");

            Ok(run_list(is_json)?)
        }
        _ => Ok(()),
    }
}

fn run_add(path: &OsStr) -> Result<(), Error> {
    log::debug!("Running add with path: {}", path.to_string_lossy());
    let index = Index::open(config::get_index_config()?)?;
    let path_buf = PathBuf::from(path);
    index.add(&path_buf)?;
    Ok(())
}

fn run_search(target: &str, exclude: Option<&Path>, find_all: bool) -> Result<(), Error> {
    log::debug!("Running search with target: {}", target);

    let index = Index::open(config::get_index_config()?)?;

    if find_all {
        return Ok(printer::print_path_slice(
            &index.find_all(target, exclude)?,
        )?);
    }

    loop {
        let directory = match index.find_one(target, exclude)? {
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

fn run_list(is_json: bool) -> Result<(), Error> {
    log::debug!("Running list with raw output: {}", is_json);
    let index = Index::open(config::get_index_config()?)?;
    if is_json {
        printer::print_json(&index.list()?)
    } else {
        printer::print_human(&index.list()?)
    }
}

fn run_init(target: &OsStr) -> Result<(), Error> {
    log::debug!("Running init with shell: {}", target.to_string_lossy());
    let shell = Shell::try_from(target)?;
    Ok(init::init_shell(shell)?)
}
