use std::{hash::Hash, path::PathBuf};

use clap::{builder::Str, command, Parser};
use hashbrown::HashMap;

use crate::utils::get_cwd;

mod db;
mod files;
mod utils;

#[derive(Parser, Debug)]
#[command(name="ynk", author="Ishan Joshi <noobscience@duck.com>", version, about="Copy paste files in the terminal", long_about = None)]

/// The Args struct is used to parse the command line arguments
/// In order to make the command line arguments more user friendly
/// the user has the option to not pass in the command name
/// If the user does not pass in the command name, then the program
/// will prompt the user to enter the command name
struct Args {
    #[arg(required = false)]
    files: Option<Vec<String>>,

    #[arg(required = false, short, long, default_value_t = false)]
    strict: bool,

    #[arg(required = false, short, long, default_value_t = true)]
    respect_ignore: bool,
}

fn main() {
    let args = Args::parse();

    // Cause I don't know a better way to do this
    #[allow(unused_assignments)]
    let mut files: HashMap<String, PathBuf> = HashMap::new();

    if let Some(req) = args.files {
        let mut raw_files: HashMap<String, PathBuf> = HashMap::new();

        req.iter().for_each(|x| {
            if !utils::does_file_exist(x) {
                bunt::println!(
                    "{$red}File or directory with path \"{$white}{}{/$}\" does not exist.{/$}",
                    x
                );
                std::process::exit(1);
            }

            if utils::is_dir(x) {
                raw_files.extend(
                    utils::list_dir(x, args.respect_ignore, false, args.strict, true)
                        .iter()
                        .map(|y| {
                            let file_name = y.to_str().unwrap().to_string();
                            (
                                utils::strip_weird_stuff(&file_name),
                                y.canonicalize().unwrap().to_path_buf(),
                            )
                        }),
                );
            } else {
                raw_files.insert(
                    utils::strip_weird_stuff(x),
                    PathBuf::from(x).canonicalize().unwrap(),
                );
            }
        });

        files = raw_files;
    } else {
        bunt::println!("{$red}No files or directory specified.{/$}");
        let file_choice =
            inquire::Confirm::new("Do you want to copy all files in the current directory?")
                .prompt()
                .unwrap();

        if file_choice {
            files = utils::list_dir(".", args.respect_ignore, false, args.strict, true)
                .iter()
                .map(|x| {
                    let file_name = x.to_str().unwrap().to_string();
                    (file_name, x.canonicalize().unwrap().to_path_buf())
                })
                .collect();
            bunt::println!("{$green}Copying all files in the current directory.{/$}");
        } else {
            bunt::println!("{$red}Please specify the files you want to copy.{/$}");
            std::process::exit(1);
        }
    }

    let conn = db::connect_to_db().expect("Could not connect to database");

    db::prep_db(&conn).expect("Could not prepare database");

    let mut entries: HashMap<String, PathBuf> = HashMap::new();

    utils::construct_entry_builders(files).iter().for_each(|x| {
        let entry =
            db::insert_into_db(&conn, x.to_owned()).expect("Could not insert into database");
        entries.insert(entry.name, PathBuf::from(entry.path));
    });

    println!("{:?}", entries);
}
