//! Some utility functions
//! that are used throughout the codebase

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use colored::Colorize;
use hashbrown::HashMap;
use ignore::{WalkBuilder, WalkState};
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
        .hidden(config.hidden)
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
pub fn construct_entry_builders(map: &HashMap<String, PathBuf>) -> Vec<EntryBuilder> {
    let mut builders = Vec::new();

    for (name, path) in map {
        let builder = EntryBuilder::new(name, path.to_str().unwrap(), path.is_dir());
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
    let path = Path::new(path);
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
        println!("A new version of ynk is available:: {}",
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
