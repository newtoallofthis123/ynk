//! Some utility functions
//! that are used throughout the codebase

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use hashbrown::HashMap;
use ignore::{WalkBuilder, WalkState};

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
pub fn list_dir(dir_path: &str, config: &ListDirConfig) -> Vec<PathBuf> {
    let paths = Arc::new(Mutex::new(Vec::new()));

    WalkBuilder::new(dir_path)
        .hidden(config.hidden)
        .git_ignore(config.respect_ignore)
        .build_parallel()
        .run(|| {
            let paths = Arc::clone(&paths);

            Box::new(move |entry| {
                let entry = if config.strict {
                    match entry {
                        Ok(entry) => entry,
                        Err(err) => {
                            bunt::println!("{$red}Error accessing entry: {:?}{/$}", err);
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

                WalkState::Continue
            })
        });

    // Extract paths from the Mutex
    Arc::try_unwrap(paths)
        .expect("Failed to unwrap Arc")
        .into_inner()
        .expect("Failed to extract paths from Mutex")
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

pub fn builder_from_entry(entry: &Entry) -> EntryBuilder {
    EntryBuilder {
        name: entry.name.clone(),
        path: entry.path.clone(),
        is_dir: entry.is_dir,
    }
}

/// Strip weird stuff from a path
/// like ./ and /
pub fn strip_weird_stuff(path: &str) -> String {
    let mut path = path.to_string();

    if path.starts_with("./") {
        path = path.strip_prefix("./").unwrap().to_string();
    }

    if path.starts_with('/') {
        path = path.strip_prefix('/').unwrap().to_string();
    }

    path
}

/// Directly print a cool splash screen
/// Still a WIP
pub fn _print_splash_screen() {
    bunt::println!("{$blue}+-+-+-+{/$}");
    bunt::println!("{$green}Y{$red}(a){/$}NK{/$}");
    bunt::println!("{$yellow}+-+-+-+{/$}");
}

/// Checks if a directory is a git repo
pub fn is_git_repo(path: &str) -> bool {
    let path = PathBuf::from(path);
    let git_path = path.join(".git");

    git_path.exists()
}

pub fn parse_range(range: &str) -> (usize, usize) {
    let range = range.split(':').collect::<Vec<&str>>();

    let start = range[0].parse::<usize>().unwrap();
    let end = range[1].parse::<usize>().unwrap();

    (start, end)
}
