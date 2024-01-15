use std::{path::PathBuf, vec};

use clap::{command, Parser};
use hashbrown::HashMap;
use tokio::task;

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
    cmd: Option<String>,

    #[arg(required = false)]
    files: Option<Vec<String>>,

    #[arg(required = false, short, long, default_value_t = false)]
    dir: bool,

    #[arg(required = false, short, long, default_value_t = false)]
    strict: bool,

    #[arg(required = false, short, long, default_value_t = true)]
    respect_ignore: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Add,
    Paste,
    Empty,
}

/// Directly print a cool splash screen
pub fn print_splash_screen() {
    bunt::println!("{$blue}+-+-+-+-+-+-+{/$}");
    bunt::println!("{$green}🗄️ Y(a)NK{/$}");
    bunt::println!("{$yellow}+-+-+-+-+-+-+{/$}");
}

#[tokio::main]
async fn main() {
    print_splash_screen();
    let args = Args::parse();
    let cmd = match args.cmd.unwrap_or_default().as_str() {
        "" => Command::Add,
        "add" => Command::Add,
        "paste" => Command::Paste,
        _ => Command::Empty,
    };

    // check all the paths
    files::check_paths_exist();

    let conn = db::connect_to_db().expect("Could not connect to database");

    db::prep_db(&conn).expect("Could not prepare database");

    let mut files: HashMap<String, PathBuf> = HashMap::new();

    if cmd == Command::Add {
        // make sure that the files are empty
        // before adding new files
        files.clear();

        let req = args.files.unwrap_or_else(|| {
            bunt::println!("{$yellow}No files or directories specified{/$}");
            bunt::println!("Copying the current directory");
            let choice = inquire::Confirm::new("Do you want to continue?")
                .with_default(true)
                .prompt()
                .unwrap();

            if !choice {
                std::process::exit(0);
            }

            vec![".".to_string()]
        });
        req.iter().for_each(|x| {
            if !utils::does_file_exist(x) {
                bunt::println!(
                    "{$red}File or directory with path \"{$white}{}{/$}\" does not exist.{/$}",
                    x
                );
                std::process::exit(1);
            }

            if utils::is_dir(x) || args.dir {
                files.extend(
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
                files.insert(
                    utils::strip_weird_stuff(x),
                    PathBuf::from(x).canonicalize().unwrap(),
                );
            }
        });

        let entries = utils::construct_entry_builders(&files)
            .iter()
            .map(|x| {
                db::insert_into_db(&conn, x.to_owned()).expect("Could not insert into database")
            })
            .collect::<Vec<_>>();

        // clear the files and entries hashmap
        files.clear();

        bunt::println!("Copied {$green}{}{/$} files", entries.len());
    } else if cmd == Command::Paste {
        files = db::get_all(&conn)
            .expect("Could not get entries from database")
            .iter()
            .map(utils::wrap_from_entry)
            .collect::<HashMap<_, _>>();

        let tasks = files.iter().map(|(name, path)| {
            let target_file = PathBuf::from(name); // Assuming name is a String or &str

            // Spawn a new asynchronous task for each file copy operation
            task::spawn(copy_paste(path.clone(), target_file.clone()))
        });

        match futures::future::try_join_all(tasks).await {
            Ok(res) => {
                let mut count = 0;

                res.iter().for_each(|x| {
                    if let Err(e) = x {
                        bunt::println!("{$red}Failed to paste file: {:?}{/$}\nUse the {$white}-v{/$} flag to see the error", e);
                    } else {
                        count += 1;
                    }
                });
                bunt::println!("Pasted {$green}{}{/$} files", count);

                if files.len() != count {
                    bunt::println!(
                        "{$red}Failed to paste {$white}{}{/$} files{/$}",
                        files.len() - count
                    );
                }
            }
            Err(e) => {
                bunt::println!("{$red}Failed to paste files: {:?}{/$}\nUse the {$white}-v{/$} flag to see the error", e);
            }
        }
    }
}

async fn copy_paste(source: PathBuf, target: PathBuf) -> Result<(), std::io::Error> {
    tokio::fs::create_dir_all(target.parent().unwrap()).await?;

    let contents = tokio::fs::read(source).await?;

    tokio::fs::write(target.clone(), contents.clone()).await?;

    Ok(())
}
