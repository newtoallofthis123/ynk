use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use hashbrown::HashMap;
use ignore::{WalkBuilder, WalkState};

use crate::db::{Entry, EntryBuilder};

pub fn does_file_exist(path: &str) -> bool {
    let path_buf = PathBuf::from(path);
    path_buf.exists()
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
pub fn list_dir(
    dir_path: &str,
    respect_ignore: bool,
    full_path: bool,
    strict: bool,
    filter_file: bool,
) -> Vec<PathBuf> {
    let paths = Arc::new(Mutex::new(Vec::new()));

    WalkBuilder::new(dir_path)
        .hidden(respect_ignore) // Adjust this based on your requirements
        .build_parallel()
        .run(|| {
            let paths = Arc::clone(&paths);

            Box::new(move |entry| {
                let entry = if strict {
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

                if filter_file && !entry.file_type().unwrap().is_file() {
                    return WalkState::Continue;
                }

                let mut paths = paths.lock().unwrap();

                if full_path {
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
        let builder = EntryBuilder::new(name, path.to_str().unwrap());
        builders.push(builder);
    }

    builders
}

pub fn wrap_from_entry(entry: &Entry) -> (String, PathBuf) {
    (entry.name.clone(), PathBuf::from(entry.path.clone()))
}

/// Checks if the given path is a directory
/// or not
pub fn is_dir(path: &str) -> bool {
    let path_buf = PathBuf::from(path);
    path_buf.is_dir()
}

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
