//! This module contains functions related to files and directories
//! It has functions to get the store and config paths

use std::path::PathBuf;

use dirs::{config_dir, home_dir};

const NAME: &str = "ynk";

/// This function returns the path to the store directory
/// ie the directory where all the files related to ynk are stored
///
/// # Panics
///
/// This function panics if it fails to get the home directory
pub fn get_store_path() -> PathBuf {
    let home_path = home_dir().expect("Failed to get home directory");
    home_path.join(PathBuf::from(".".to_owned() + NAME))
}

/// This function checks if the store and config directories exist
/// If they don't, it creates them
///
/// # Panics
///
/// This function panics if it fails to create the directories
/// or when it fails to get the necessary paths
pub fn check_paths_exist() {
    let store_path = get_store_path();
    if !store_path.exists() {
        std::fs::create_dir_all(store_path).expect("Failed to create store directory");
    }

    let config_path = get_config_path();
    if !config_path.exists() {
        std::fs::create_dir_all(config_path).expect("Failed to create config directory");
    }
}

/// This function returns the path to the config file
pub fn get_config_path() -> PathBuf {
    let config_path = config_dir()
        .expect("Failed to get config directory")
        .join(NAME);
    config_path.join("./config.toml")
}

/// This function returns the path to the file in the store directory
/// basically it joins the path passed to it with the store directory
pub fn get_path(path: &str) -> PathBuf {
    let path_buf = get_store_path();
    path_buf.join(path)
}
