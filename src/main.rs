use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
    vec,
};

use clap::{command, Parser};
use hashbrown::HashMap;
use indicatif::ProgressBar;
use tokio::{sync::Mutex, task};
use utils::{list_dir, ListDirConfig};

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

    #[arg(required = false, short, long)]
    dir: bool,

    #[arg(required = false, short, long)]
    strict: bool,

    #[arg(required = false, short, long)]
    no_ignore: bool,

    #[arg(required = false, long)]
    hidden: bool,

    #[arg(required = false, long)]
    dry_run: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Add,
    Paste,
    Empty,
}

#[tokio::main]
async fn main() {
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

            files.insert(
                utils::strip_weird_stuff(x),
                PathBuf::from(x).canonicalize().unwrap(),
            );
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

        static LIST_DIR_CONFIG: OnceLock<ListDirConfig> = OnceLock::new();
        LIST_DIR_CONFIG.get_or_init(|| ListDirConfig {
            filter_file: !args.dir,
            full_path: false,
            strict: args.strict,
            hidden: args.hidden,
            respect_ignore: !args.no_ignore,
        });

        let mut final_files = HashMap::new();

        files.iter().for_each(|(name, path)| {
            if path.is_dir() {
                bunt::println!("{$yellow}Target is a directory{/$}");
                let entries = list_dir(path.to_str().unwrap(), LIST_DIR_CONFIG.get().unwrap());
                final_files.extend(entries.iter().map(|x| {
                    let (name, path) = utils::wrap_from_path(path, x);
                    (name, path)
                }));
            } else {
                final_files.insert(name.clone(), path.clone());
            }
        });

        let pb = Arc::new(Mutex::new(ProgressBar::new_spinner()));

        let tasks = final_files.iter().map(|(name, path)| {
            let target_file = PathBuf::from(name); // Assuming name is a String or &str
            let pb_clone = Arc::clone(&pb);

            // Spawn a new asynchronous task for each file copy operation
            task::spawn(copy_paste(
                pb_clone,
                path.clone(),
                target_file.clone(),
                args.dry_run,
            ))
        });

        match futures::future::try_join_all(tasks).await {
            Ok(res) => {
                let mut count: u64 = 0;

                res.iter().for_each(|x| {
                    if let Err(e) = x {
                        bunt::println!("{$red}Failed to paste file: {:?}{/$}\nUse the {$white}-v{/$} flag to see the error", e);
                    } else{
                        count += 1;
                    }
                });

                match db::delete_all(&conn) {
                    Ok(_) => {}
                    Err(e) => {
                        bunt::println!("{$red}Failed to delete all entries from database: {:?}{/$}\nUse the {$white}-v{/$} flag to see the error", e);
                    }
                }

                let pb = pb.lock().await;
                pb.finish_with_message(format!("Pasted {} files", count));
            }
            Err(e) => {
                bunt::println!("{$red}Failed to paste files: {:?}{/$}\nUse the {$white}-v{/$} flag to see the error", e);
            }
        }
    }
}

/// The Async function in charge of copying and pasting files
/// from the source to the target
/// This is at the core of the program
/// So, essentially, this function acts as an async and completely
/// parallelized version of the `cp` command
async fn copy_paste(
    pb: Arc<Mutex<ProgressBar>>,
    source: PathBuf,
    target: PathBuf,
    dry_run: bool,
) -> Result<(), std::io::Error> {
    tokio::fs::create_dir_all(target.parent().unwrap()).await?;

    let contents = tokio::fs::read(source.clone()).await?;

    if !dry_run {
        tokio::fs::write(target.clone(), contents.clone()).await?;
    }
    let pb = pb.lock().await;
    pb.inc(1);

    Ok(())
}
