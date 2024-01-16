use std::path::PathBuf;

use clap::{command, Parser};
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
    #[arg(required = false)]
    cmd: Option<String>,

    #[arg(required = false)]
    files: Option<Vec<String>>,

    #[arg(required = false, short, long)]
    dir: bool,

    #[arg(required = false, short, long)]
    strict: bool,

    #[arg(required = false, short, long)]
    no_ignore: bool,

    #[arg(required = false, long)]
    hidden: bool,

    #[arg(required = false, short, long)]
    overwrite: bool,

    #[arg(required = false, short, long)]
    target: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Add,
    Paste,
    Pop,
    List,
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

    handler::handler(cmd, args, &conn).await;
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
