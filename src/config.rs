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
/// This takes up a bit more memory than the Arg struct
/// but it is worth it in the long run
#[derive(Debug, Clone)]
pub struct ConstructedArgs {
    pub files: Option<Vec<String>>,
    pub dir: bool,
    pub strict: bool,
    pub no_ignore: bool,
    pub hidden: bool,
    pub overwrite: bool,
    pub delete: bool,
    pub range: Option<String>,
    pub specific: Option<String>,
}

impl ConstructedArgs {
    pub fn new(args: Args, config: Config) -> Self {
        Self {
            files: args.files,
            dir: args.dir,
            strict: arg_or_config(args.strict, config.strict),
            hidden: arg_or_config(args.hidden, config.hidden),
            overwrite: arg_or_config(args.overwrite, config.overwrite),
            no_ignore: arg_or_config(args.no_ignore, config.no_ignore),
            delete: arg_or_config(args.delete, config.delete),
            range: args.range,
            specific: None,
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

fn default_config() -> String {
    r#"
strict = false
no_ignore = false
hidden = false
overwrite = false
delete = false
"#
    .to_string()
}

#[derive(serde::Deserialize, Debug)]
pub struct Config {
    pub strict: bool,
    pub no_ignore: bool,
    pub hidden: bool,
    pub overwrite: bool,
    pub delete: bool,
}

/// Convert config from string to Config struct
pub fn get_config(config: String) -> Config {
    let config: Config =
        toml::from_str(config.as_str()).unwrap_or(toml::from_str(&default_config()).unwrap());
    config
}

pub fn get_config_from_file() -> Config {
    let config_path = get_config_path();
    let config = std::fs::read_to_string(config_path).unwrap_or(default_config());
    get_config(config)
}

/// Writes the default config to the config file
pub fn write_default_config() {
    write_file(&get_config_path(), default_config());
}
