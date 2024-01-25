use std::path::PathBuf;

use clap::{command, Parser};
use config::{get_config_from_file, write_default_config, ConstructedArgs};
use files::get_config_path;
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

    #[arg(required = false, long, help = "Include hidden files while pasting")]
    hidden: bool,

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
        help = "Delete the file from the store, but not from the disk"
    )]
    delete: bool,

    #[arg(
        required = false,
        short,
        long,
        help = "The Range of files to be pasted"
    )]
    range: Option<String>,
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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if !get_config_path().exists() {
        write_default_config();
    }

    let config = get_config_from_file();

    let temp_arg = args.clone();
    let mut cmd = match args.clone().cmd {
        Some(cmd) => Command::from(&cmd),
        None => {
            bunt::println!("{$yellow}Interactive Mode{/$}");
            get_cmd()
        }
    };

    if cmd == Command::Empty {
        if let Some(cmd) = temp_arg.cmd {
            if PathBuf::from(cmd.clone()).exists() {
                bunt::println!("You seem to have entered a {$red}file path{/$}");
                bunt::println!("You can use {$blue}ynk add {}{/$} to add to the store", cmd);
                std::process::exit(0);
            }
        } else {
            bunt::println!(
                "{$red}Invalid Command{/$} \"{$green}{}{/$}\"",
                &temp_arg.cmd.unwrap()
            );
        }
        cmd = get_cmd();
    }

    // check all the paths
    files::check_paths_exist();

    let conn = db::connect_to_db().expect("Could not connect to database");

    db::prep_db(&conn).expect("Could not prepare database");

    let constructed_args = ConstructedArgs::new(args, config);

    handler::handler(cmd, constructed_args, &conn).await;
}

fn get_cmd() -> Command {
    let choice = inquire::Select::new(
        "Select a Command",
        vec!["add", "paste", "list", "clear", "pop", "exit"],
    )
    .prompt()
    .unwrap();

    Command::from(choice)
}
