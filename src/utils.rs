//! Some utility functions
//! that are used throughout the codebase

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use clap::{command, Arg, ArgAction, Command};
use colored::Colorize;
use correct_word::levenshtein::levenshtein_distance;
use hashbrown::{HashMap, HashSet};
use ignore::{WalkBuilder, WalkState};
use path_abs::PathInfo;
use update_informer::{registry, Check};

use crate::db::{Entry, EntryBuilder};

pub fn does_file_exist(path: &str) -> bool {
    let path_buf = PathBuf::from(path);
    path_buf.exists()
}

/// The Config struct that is used to configure the list_dir function
/// Easier than setting all the arguments
/// Plus, is usually OnceLocked
/// so, it can be used in multiple threads
#[derive(Debug, Clone)]
pub struct ListDirConfig {
    pub respect_ignore: bool,
    pub full_path: bool,
    pub strict: bool,
    pub hidden: bool,
    pub filter_file: bool,
}

/// Recursively lists all the files and directories in a directory
/// using the effective ignore file
/// as outlined by the crate `ignore`
/// which implements good algorithms listing and ignoring files
///
/// # Warning
///
/// This function does not throw any errors
/// when it encounters an error while listing the files
/// so, if you want to handle errors, use `strict_list_dir`
///
/// # Arguments
///
/// * `dir_path` - The path of the directory to list
/// * `respect_ignore` - Whether to respect the ignore file or not
/// * `full_path` - Whether to return the full path or not
/// * `strict` - Whether to throw errors when it encounters an error while listing files
///
/// Usually, you would want to set the respect_ignore to true
/// and full_path to true.
/// Since I can't really set defaults for these, so think of them as
/// defaults.
///
/// # Returns
///
/// A vector of `PathBuf`s
pub fn list_dir(dir_path: &str, config: &ListDirConfig) -> (Vec<PathBuf>, f64) {
    let paths = Arc::new(Mutex::new(Vec::new()));
    let size: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));

    WalkBuilder::new(dir_path)
        .hidden(!config.hidden)
        .git_ignore(config.respect_ignore)
        .build_parallel()
        .run(|| {
            let paths = Arc::clone(&paths);
            let size = Arc::clone(&size);

            Box::new(move |entry| {
                let entry = if config.strict {
                    match entry {
                        Ok(entry) => entry,
                        Err(err) => {
                            println!("{}: {:?}", "Error accessing entry:".red(), err);
                            return WalkState::Quit;
                        }
                    }
                } else {
                    match entry {
                        Ok(entry) => entry,
                        Err(_) => return WalkState::Continue,
                    }
                };

                if config.filter_file && !entry.file_type().unwrap().is_file() {
                    return WalkState::Continue;
                }

                let mut paths = paths.lock().unwrap();

                if config.full_path {
                    paths.push(entry.path().canonicalize().unwrap());
                } else {
                    paths.push(entry.path().to_path_buf());
                }

                // add to size
                let mut size = size.lock().unwrap();
                *size += entry.metadata().unwrap().len();

                WalkState::Continue
            })
        });

    // convert to kb
    let size = *size.lock().unwrap() as f64 / 1024.0;

    // Extract paths from the Mutex
    (
        Arc::try_unwrap(paths)
            .expect("Failed to unwrap Arc")
            .into_inner()
            .expect("Failed to extract paths from Mutex"),
        size,
    )
}

/// Constructs a vector of `EntryBuilder`s
/// from a `HashMap` of `PathBuf`s
pub fn construct_entry_builders(map: &HashMap<String, PathBuf>, is_dir: bool) -> Vec<EntryBuilder> {
    let mut builders = Vec::new();

    for (name, path) in map {
        let builder = EntryBuilder::new(name, path.to_str().unwrap(), is_dir);
        builders.push(builder);
    }

    builders
}

pub fn wrap_from_entry(entry: &Entry) -> (String, PathBuf) {
    (entry.name.clone(), PathBuf::from(entry.path.clone()))
}

pub fn wrap_from_path(root: &Path, path: &Path) -> (String, PathBuf) {
    (
        path.strip_prefix(root)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
        path.to_path_buf(),
    )
}

/// Parses the file name from a path
pub fn parse_file_name(path: &str) -> String {
    let path = Path::new(path).canonicalize().unwrap();
    let file_name = path.file_name().unwrap().to_str().unwrap();

    file_name.to_string()
}

/// Directly print a cool splash screen
pub fn print_splash_screen() {
    println!("{}", "+-+-+-+".blue());
    println!("{}", "Y(a)NK".green());
    println!("{}", "+-+-+-+".blue());
}

/// Checks if a directory is a git repo
pub fn _is_git_repo(path: &str) -> bool {
    let path = PathBuf::from(path);
    let git_path = path.join(".git");

    git_path.exists()
}

pub fn check_version() {
    let pkg_name = env!("CARGO_PKG_NAME");
    let current_version = env!("CARGO_PKG_VERSION");

    let informer = update_informer::new(registry::Crates, pkg_name, current_version);
    if let Some(version) = informer.check_version().unwrap() {
        println!(
            "A new version of ynk is available:: {}",
            version.to_string().yellow()
        );
    }
}

/// Converts the size from bytes to human readable string
/// Borrowed from https://github.com/banyan/rust-pretty-bytes
pub fn convert_size(num: f64) -> String {
    let negative = if num.is_sign_positive() { "" } else { "-" };
    let num = num.abs();
    let units = ["kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    if num < 1_f64 {
        return format!("{}{} {}", negative, num, "B");
    }
    let delimiter = 1000_f64;
    let exponent = std::cmp::min(
        (num.ln() / delimiter.ln()).floor() as i32,
        (units.len() - 1) as i32,
    );
    let pretty_bytes = format!("{:.2}", num / delimiter.powi(exponent))
        .parse::<f64>()
        .unwrap()
        * 1_f64;
    let unit = units[exponent as usize];
    format!("{}{} {}", negative, pretty_bytes, unit)
}

pub fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| b.id.cmp(&a.id));
}

pub fn deep_search(queries: Vec<String>, entries: &[Entry]) -> Vec<Entry> {
    let mut res = HashSet::new();

    if queries.is_empty() {
        return entries.to_vec();
    }

    for query in queries {
        let mut query = query;
        if PathBuf::from(query.clone()).exists() {
            query = PathBuf::from(query.clone())
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .to_string();
        }
        entries.iter().for_each(|e| {
            let leven_dis = levenshtein_distance(query.to_string(), e.name.clone());
            let dis = 1.0 - (leven_dis as f64 / std::cmp::max(query.len(), e.name.len()) as f64);
            if let Ok(id) = query.parse::<i32>() {
                res.insert(id);
            }
            if query == e.name
                || query == e.path
                || e.name.starts_with(&query)
                || e.path.starts_with(&query)
                || dis >= 0.5
            {
                res.insert(e.id);
            }
        });
    }

    entries
        .iter()
        .filter(|y| res.contains(&y.id))
        .cloned()
        .collect()
}

pub fn setup_cli() -> Command {
    command!()
        .author("NoobScience <noobscience@duck.com>")
        .about("Copy paste files in the terminal")
        .arg(
            Arg::new("noignore")
                .short('n')
                .help("Don't respect the .gitignore")
                .long("noignore")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("yes")
                .short('y')
                .help("Prompt yes to all prompts")
                .long("yes")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("all")
                .short('a')
                .long("all")
                .help("Also include hidden files in discovery")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("list").arg(
                Arg::new("size")
                    .id("size")
                    .long("size")
                    .short('s')
                    .help("Calculate and show the size column")
                    .action(ArgAction::SetTrue),
            ).long_about("List the entires in the store"),
        )
        .subcommand(
            Command::new("add")
                .arg(
                    Arg::new("dir")
                        .help("Set sign as a dir")
                        .long("dir")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("preserve-structure")
                        .long("preserve")
                        .help("Preserves the dir structure")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("files")
                        .help("The list of files to add")
                        .num_args(1..)
                        .value_name("FILES"),
                ).long_about("Add entries to the store"),
        )
        .subcommand(
            Command::new("delete").long_about("Delete entries from the ynk store").arg(
                Arg::new("queries")
                    .help("The queries to file the entries")
                    .num_args(1..)
                    .value_name("QUERIES"),
            ),
        )
        .subcommand(
            Command::new("pop")
                .long_about("Pop the last entry in the ynk store")
                .arg(
                    Arg::new("overwrite")
                        .long("overwrite")
                        .help("Overwrite existing files")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("The output dir or file")
                        .num_args(1)
                )
                .arg(Arg::new("strict").help("Error on any IO error").long("strict").action(ArgAction::SetTrue)),
        )
        .subcommand(Command::new("clear").long_about("Clear all entries from the ynk store"))
        .subcommand(
            Command::new("paste")
                .long_about("Paste entries from the ynk store")
                .arg(
                    Arg::new("overwrite")
                        .long("overwrite")
                        .help("Overwrite existing files")
                        .action(ArgAction::SetTrue),
                )
                .arg(Arg::new("strict").help("Error on any IO error").long("strict").action(ArgAction::SetTrue))
                .arg(
                    Arg::new("delete")
                        .long("delete")
                        .help("Delete the entry from the store after pasting")
                        .short('d')
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("The output dir or file")
                        .num_args(1)
                )
                .arg(
                    Arg::new("queries")
                        .help("Queries to filter the entires")
                        .num_args(1..)
                        .value_name("QUERIES")
                )
                .arg(
                    Arg::new("range").long("range").help("Specify the range of entries to paste: Works using the syntax of n..[m]").short('r').num_args(1)
                ),
        ).subcommand(Command::new("completions")
                .arg(
                    Arg::new("shell")
                        .help("The list of files to add")
                        .num_args(1)
                        .value_name("SHELL")
                        .required(true)
                ).long_about("Generate and write completions")
        )
}
