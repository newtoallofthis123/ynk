//! This module contains the config related functions

use std::path::Path;

use crate::{files::get_config_path, Args};

/// ConstructedArgs struct
/// which is used to emulate or mimic
/// Arg struct
///
/// This is a safe way to interact with the async paste
/// handler without accidentally messing up the arguments
///
/// Moreover, this is a good way to pass arguments from
/// the config file to the async paste handler
///
/// # Warning
///
/// In the actual config implementation, every argument
/// is placed in a "OR" relationship with the arguments passed
/// to the command line.
///
/// # Note
///
/// This takes up a bit more memory than the Arg struct,
/// but it is worth it in the long run
pub struct ConstructedArgs {
    pub files: Option<Vec<String>>,
    pub dir: bool,
    pub strict: bool,
    pub no_ignore: bool,
    pub all: bool,
    pub overwrite: bool,
    pub delete: bool,
    pub range: Option<String>,
    pub specific: Option<String>,
    pub yes: bool,
    pub calculate_size: bool,
    pub preserve_structure: bool,
}

impl ConstructedArgs {
    pub fn new(args: Args, config: Config) -> Self {
        Self {
            files: args.files,
            dir: args.dir,
            strict: arg_or_config(args.strict, config.strict),
            all: arg_or_config(args.all, config.all),
            overwrite: arg_or_config(args.overwrite, config.overwrite),
            no_ignore: arg_or_config(args.no_ignore, config.no_ignore),
            delete: arg_or_config(args.delete, config.delete),
            range: args.range,
            calculate_size: arg_or_config(args.size, config.calculate_size),
            specific: None,
            yes: arg_or_config(args.yes, config.prompt),
            preserve_structure: arg_or_config(args.preserve_structure, config.preserve_structure),
        }
    }
}

pub fn arg_or_config(to_check: bool, replace: bool) -> bool {
    if !to_check {
        replace
    } else {
        to_check
    }
}

/// Write a file to the specified path
pub fn write_file(path: &Path, content: String) -> bool {
    let _ = std::fs::write(path, content);
    true
}

fn default_config() -> Result<String, toml::ser::Error> {
    let config = Config {
        strict: false,
        no_ignore: false,
        all: false,
        overwrite: false,
        delete: false,
        prompt: true,
        show_splash: true,
        calculate_size: true,
        preserve_structure: false,
    };

    toml::to_string_pretty(&config)
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Config {
    pub strict: bool,
    pub no_ignore: bool,
    pub all: bool,
    pub overwrite: bool,
    pub delete: bool,
    pub prompt: bool,
    pub show_splash: bool,
    pub calculate_size: bool,
    pub preserve_structure: bool,
}

/// Convert config from string to Config struct
pub fn get_config(config: String) -> Config {
    let default_config = default_config().expect("Failed to serialize default config");

    let config: Config =
        toml::from_str(config.as_str()).unwrap_or(toml::from_str(default_config.as_str()).unwrap());
    config
}

pub fn get_config_from_file() -> Config {
    let config_path = get_config_path();
    let default_config = default_config().expect("Failed to serialize default config");

    let config = std::fs::read_to_string(config_path).unwrap_or(default_config);
    get_config(config)
}

/// Writes the default config to the config file
pub fn write_default_config() {
    let default_config = default_config().expect("Failed to serialize default config");

    write_file(&get_config_path(), default_config);
}
