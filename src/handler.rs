use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use hashbrown::HashMap;
use indicatif::ProgressBar;
use tokio::{sync::Mutex, task};

use crate::{
    db,
    utils::{self, does_file_exist, list_dir, ListDirConfig},
    Args, Command,
};

pub async fn handler(cmd: Command, args: Args, conn: &rusqlite::Connection) {
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
            if !does_file_exist(x) {
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
                db::insert_into_db(conn, x.to_owned()).expect("Could not insert into database")
            })
            .collect::<Vec<_>>();

        // clear the files and entries hashmap
        files.clear();

        bunt::println!("Copied {$green}{}{/$} files", entries.len());
    } else if cmd == Command::Paste {
        let paste_config = PasteBuilder {
            files: args.files,
            dir: args.dir,
            strict: args.strict,
            no_ignore: args.no_ignore,
            hidden: args.hidden,
            dry_run: args.dry_run,
        };

        handle_paste(paste_config, conn).await;
    } else if cmd == Command::Exit {
        bunt::println!("{$yellow}Bye!{/$}");
        std::process::exit(0);
    } else if cmd == Command::List {
        let entries = db::get_all(conn).expect("Could not get entries from database");

        bunt::println!("{$green}{}{/$} files in store", entries.len());

        entries.iter().for_each(|x| {
            bunt::println!("{$blue}{}{/$}", x.path);
        });

        bunt::println!("Use {$green}ynk paste{/$} to paste the files");
    } else if cmd == Command::Pop {
        let entry = db::pop_one(conn).expect("Could not pop entry from database");

        let paste_config = PasteBuilder {
            files: Some(vec![entry.name]),
            dir: args.dir,
            strict: false,
            no_ignore: args.no_ignore,
            hidden: args.hidden,
            dry_run: args.dry_run,
        };

        handle_paste(paste_config, conn).await;
    } else if cmd == Command::Clear {
        let choice = inquire::Confirm::new("Are you sure you want to clear all the copied files?")
            .prompt()
            .unwrap();

        if !choice {
            bunt::println!("Ok! {$red}Quitting{/$}");
        }

        bunt::println!("Clearing the indexed files");
        db::delete_all(conn).expect("Unable to delete the indexes");
    } else if cmd == Command::Delete {
        let entries = db::get_all(conn).expect("Could not get entries from database");

        let mut choices = entries
            .iter()
            .map(utils::wrap_from_entry)
            .collect::<HashMap<_, _>>();

        choices.insert("exit".to_string(), PathBuf::from("exit"));

        let mut to_delete = Vec::new();

        loop {
            let choice = inquire::Select::new(
                "Select a file to delete",
                choices.iter().map(|(n, _)| n).collect::<Vec<_>>(),
            )
            .prompt()
            .unwrap();

            if choice == "exit" {
                break;
            }

            to_delete.push(choices.get(choice).unwrap().clone());
        }

        to_delete.iter().for_each(|x| {
            db::delete_entry(conn, x.to_str().unwrap()).expect("Unable to delete entry");
        });

        bunt::println!("Deleted {$green}{}{/$} files", to_delete.len());
    }
}

/// Private PasteBuilder struct
/// which is used to emulate or mimic
/// Arg struct
///
/// This is a safe way to interact with the async paste
/// handler without accidentally messing up the arguments
///
/// # Note
///
/// This takes up a bit more memory than the Arg struct
/// but it is worth it in the long run
struct PasteBuilder {
    files: Option<Vec<String>>,
    dir: bool,
    strict: bool,
    no_ignore: bool,
    hidden: bool,
    dry_run: bool,
}

/// Private async function to handle the paste command
async fn handle_paste(paste_config: PasteBuilder, conn: &rusqlite::Connection) {
    let mut files = db::get_all(conn)
        .expect("Could not get entries from database")
        .iter()
        .map(utils::wrap_from_entry)
        .collect::<HashMap<_, _>>();

    if paste_config.files.is_some() {
        let temp_files = files
            .iter()
            .filter(|(name, _)| paste_config.files.as_ref().unwrap().contains(name))
            .map(|(name, path)| (name.clone(), path.clone()))
            .collect::<HashMap<_, _>>();

        if temp_files.is_empty() {
            bunt::println!("{$yellow}Specified files do not exist{/$}");
            bunt::println!("use {$green}ynk add{/$} to add files to the store");
            std::process::exit(1);
        } else {
            files = temp_files;
        }
    }

    files.iter().for_each(|(_, path)| {
        db::delete_entry(conn, path.to_str().unwrap()).expect("Unable to delete entry");
    });

    static LIST_DIR_CONFIG: OnceLock<ListDirConfig> = OnceLock::new();
    LIST_DIR_CONFIG.get_or_init(|| ListDirConfig {
        filter_file: !paste_config.dir,
        full_path: false,
        strict: paste_config.strict,
        hidden: paste_config.hidden,
        respect_ignore: !paste_config.no_ignore,
    });

    let mut final_files = HashMap::new();

    files.iter().for_each(|(name, path)| {
        if path.is_dir() {
            // bunt::println!("{$yellow}Target is a directory{/$}");
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
            paste_config.dry_run,
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

            let pb = pb.lock().await;
            pb.finish_with_message(format!("Pasted {} files", count));
        }
        Err(e) => {
            bunt::println!("{$red}Failed to paste files: {:?}{/$}\nUse the {$white}-v{/$} flag to see the error", e);
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
