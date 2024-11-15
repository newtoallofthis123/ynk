use std::path::PathBuf;

use clap::{command, Parser};
use colored::Colorize;
use config::{get_config_from_file, write_default_config, ConstructedArgs};
use correct_word::{correct_word, Algorithm};
use files::get_config_path;
use human_panic::setup_panic;
use utils::{check_version, print_splash_screen};

mod config;
mod db;
mod files;
mod handler;
mod utils;

#[derive(Parser, Debug, Clone)]
#[command(name="ynk", author="Ishan Joshi <noobscience@duck.com>", version, about="Copy paste files in the terminal", long_about = None)]

/// The Args struct is used to parse the command line arguments
/// In order to make the command line arguments more user friendly
/// the user has the option to not pass in the command name
/// If the user does not pass in the command name, then the program
/// will prompt the user to enter the command name
struct Args {
    #[arg(required = false, help = "The command to be executed")]
    cmd: Option<String>,

    #[arg(
        required = false,
        help = "The name of the files / directories to be added to the store"
    )]
    files: Option<Vec<String>>,

    #[arg(
        required = false,
        long,
        help = "Explicitly specify that the given path is a directory"
    )]
    dir: bool,

    #[arg(required = false, short, long, help = "Throw any and all IO errors")]
    strict: bool,

    #[arg(
        required = false,
        short,
        long,
        help = "Ignore the .gitignore file while adding files"
    )]
    no_ignore: bool,

    #[arg(
        required = false,
        short,
        long,
        help = "Include hidden files while pasting"
    )]
    all: bool,

    #[arg(
        required = false,
        short,
        long,
        help = "Overwrite the file if it already exists while pasting"
    )]
    overwrite: bool,

    #[arg(
        required = false,
        short,
        long,
        help = "Delete the file from the store / clipboard after pasting it"
    )]
    delete: bool,

    #[arg(
        required = false,
        short,
        long,
        help = "The Range of files to be pasted"
    )]
    range: Option<String>,

    #[arg(required = false, short, long, help = "Don't ask for confirmation")]
    yes: bool,

    #[arg(required = false, long, help = "Show size of the files in the list")]
    size: bool,

    #[arg(
        required = false,
        long,
        help = "Preserve the Directory Structure for an individual file"
    )]
    preserve_structure: bool,

    #[arg(
        required = false,
        long,
        help = "Directly select the id from the clipboard"
    )]
    id: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Add,
    Paste,
    Pop,
    List,
    Config,
    Delete,
    Clear,
    Empty,
    Exit,
}

impl Command {
    fn from(cmd: &str) -> Command {
        match cmd {
            "add" => Command::Add,
            "paste" => Command::Paste,
            "pop" => Command::Pop,
            "config" => Command::Config,
            "clear" => Command::Clear,
            "list" => Command::List,
            "delete" => Command::Delete,
            "exit" => Command::Exit,
            _ => Command::Empty,
        }
    }
}

const OPTIONS: &[&str] = &[
    "add", "paste", "pop", "list", "config", "delete", "clear", "exit",
];

#[tokio::main]
async fn main() {
    setup_panic!();

    let args = Args::parse();

    if !get_config_path().exists() {
        write_default_config();
    }

    let config = get_config_from_file();

    if config.show_splash {
        print_splash_screen();
    }

    let temp_arg = args.clone();
    let mut cmd = match args.clone().cmd {
        Some(cmd) => Command::from(&cmd),
        None => {
            println!("{}", "Interactive Mode".red());
            get_cmd()
        }
    };

    let mut constructed_args = ConstructedArgs::new(args, config);

    if cmd == Command::Empty {
        if let Some(temp_cmd) = temp_arg.cmd {
            if PathBuf::from(temp_cmd.clone()).exists() {
                cmd = Command::Add;
                constructed_args.files = Some(vec![temp_cmd]);
            } else if !temp_cmd.is_empty() {
                let word = correct_word(
                    Algorithm::Levenshtein,
                    temp_cmd,
                    OPTIONS.iter().map(|x| x.to_string()).collect(),
                    Some(5),
                );
                if let Some(word) = word.word {
                    println!("You seem to have meant {}", word.blue());
                    if !inquire::Confirm::new("Do you want to continue?")
                        .with_default(true)
                        .prompt()
                        .unwrap()
                    {
                        std::process::exit(0);
                    }

                    cmd = Command::from(&word);
                } else {
                    cmd = get_cmd();
                }
            }
        } else {
            println!(
                "{} {}",
                "Invalid Command".red(),
                &temp_arg.cmd.unwrap().red()
            );
            cmd = get_cmd();
        }
    }

    // check all the paths
    files::check_paths_exist();

    let conn = db::connect_to_db().expect("Could not connect to database");

    db::prep_db(&conn).expect("Could not prepare database");

    check_version();

    handler::handler(cmd, constructed_args, &conn).await;
}

fn get_cmd() -> Command {
    let choice = inquire::Select::new("Select a Command", OPTIONS.to_vec())
        .prompt()
        .unwrap();

    Command::from(choice)
}
