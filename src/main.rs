// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use anyhow::Result;
use clap::{command, Arg, ArgAction, Command};
use std::convert::TryFrom;
use std::path::{Path, PathBuf};

use crate::index::{Index, IndexError};
use crate::init::Shell;

mod config;
mod index;
mod init;
mod printer;

fn main() -> Result<()> {
    pretty_env_logger::init();
    let path_arg = Arg::new("path")
        .value_name("PATH")
        .help("The path to add into the index")
        .required(true);

    let target_arg = Arg::new("target")
        .value_name("TARGET")
        .help("The target to jump to")
        .required(true);

    let exclude_arg = Arg::new("exclude")
        .value_name("PATH")
        .long("exclude")
        .short('e')
        .number_of_values(1)
        .help("Exclude the given path from the search results");

    let all_arg = Arg::new("all")
        .long("all")
        .short('a')
        .action(ArgAction::SetTrue)
        .help("Return all matched entries instead of only the most relevant one");

    let shell_arg = Arg::new("shell")
        .value_name("SHELL")
        .help(format!(
            "The shell scotty needs to integrate with. One of: {:?}",
            Shell::all_variants()
        ))
        .value_parser(parse_shell)
        .required(true);

    let json_arg = Arg::new("json")
        .long("json")
        .action(ArgAction::SetTrue)
        .help("Print output as a series of newline delimited json objects");

    let matches = command!()
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about("Transports you into a directory based on previous usage")
        .subcommand_required(true)
        .subcommand(
            Command::new("add")
                .about("Add a path to the index")
                .arg(&path_arg),
        )
        .subcommand(
            Command::new("search")
                .about("Searches a directory based on the input and the current index")
                .arg(&exclude_arg)
                .arg(&all_arg)
                .arg(&target_arg),
        )
        .subcommand(
            Command::new("init")
                .about("Integrates scotty in your shell")
                .arg(&shell_arg),
        )
        .subcommand(
            Command::new("list")
                .about("Print the current index")
                .arg(&json_arg),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("add", sub_m)) => {
            let path = sub_m.get_one::<String>("path").expect("Path is missing");

            Ok(run_add(path)?)
        }
        Some(("search", sub_m)) => {
            let target = sub_m
                .get_one::<String>("target")
                .expect("Target is missing");
            let excluded_path = sub_m.get_one::<String>("exclude").map(Path::new);
            let find_all = sub_m.get_flag("all");

            Ok(run_search(target, excluded_path, find_all)?)
        }
        Some(("init", sub_m)) => {
            let shell = sub_m.get_one("shell").expect("Shell is missing");

            Ok(run_init(shell)?)
        }
        Some(("list", sub_m)) => {
            let is_json = sub_m.get_flag("json");

            Ok(run_list(is_json)?)
        }
        _ => Ok(()), // Unreachable
    }
}

fn parse_shell(shell: &str) -> Result<Shell, init::ShellError> {
    Shell::try_from(shell)
}

fn run_add(path: &str) -> Result<()> {
    log::debug!("Running add with path: {path}");
    let index = Index::open(config::get_index_config()?)?;
    let path_buf = PathBuf::from(path);
    index.add(&path_buf)?;
    Ok(())
}

fn run_search(target: &str, exclude: Option<&Path>, find_all: bool) -> Result<()> {
    log::debug!("Running search with target: {target}");

    let index = Index::open(config::get_index_config()?)?;

    if find_all {
        return printer::print_path_slice(&index.find_all(target, exclude)?);
    }

    loop {
        let directory = match index.find_one(target, exclude)? {
            None => return Err(IndexError::NoResults(target.to_owned()).into()),
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

fn run_list(is_json: bool) -> Result<()> {
    log::debug!("Running list with raw output: {is_json}");
    let index = Index::open(config::get_index_config()?)?;
    if is_json {
        printer::print_json(&index.list()?)
    } else {
        printer::print_human(&index.list()?)
    }
}

fn run_init(shell: &Shell) -> Result<()> {
    log::debug!("Running init with shell: {shell:?}");
    Ok(init::init_shell(shell)?)
}
