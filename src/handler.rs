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
    overwrite: bool,
    target: Option<String>,
    delete: bool,
}

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
            overwrite: args.overwrite,
            target: args.target,
            delete: args.delete,
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
        let entry = match db::pop_one(conn) {
            Ok(entry) => entry,
            Err(e) => {
                bunt::println!("{$red}Could not pop entry from database: {:?}{/$}", e);
                std::process::exit(1);
            }
        };

        let paste_config = PasteBuilder {
            files: Some(vec![entry.name]),
            dir: args.dir,
            strict: args.strict,
            no_ignore: args.no_ignore,
            hidden: args.hidden,
            overwrite: args.overwrite,
            target: args.target,
            delete: args.delete,
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

/// Private async function to handle the paste command
async fn handle_paste(paste_config: PasteBuilder, conn: &rusqlite::Connection) {
    let mut files = db::get_all(conn)
        .expect("Could not get entries from database")
        .iter()
        .map(utils::wrap_from_entry)
        .collect::<HashMap<_, _>>();

    let user_target = paste_config
        .files
        .unwrap_or_else(|| vec![".".to_string()])
        .first()
        .unwrap()
        .clone();

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

    if let Some(to_get) = paste_config.target.clone() {
        files = db::get_all(conn)
            .expect("Could not get entries from database")
            .iter()
            .filter(|x| x.path.starts_with(to_get.as_str()))
            .map(utils::wrap_from_entry)
            .collect::<HashMap<_, _>>();
    }

    let pb = Arc::new(Mutex::new(ProgressBar::new_spinner()));

    if utils::is_git_repo(
        &paste_config
            .target
            .clone()
            .unwrap_or_else(|| ".".to_string()),
    ) {
        bunt::println!("{$blue}Target directory is a git repository{/$}");
        bunt::println!("This may cause some problems with memory, which may cause your system to hang while the IO is being performed");
        bunt::println!("{$yellow}Proceed with caution{/$}");

        let choice = inquire::Confirm::new("Do you want to continue?")
            .with_default(false)
            .prompt()
            .unwrap();

        if !choice {
            bunt::println!("Good choice! I'll definitely fix this in the future");
            std::process::exit(0);
        }
    }

    let tasks = final_files.iter().map(|(name, path)| {
        if !PathBuf::from(user_target.clone()).exists() {
            bunt::println!("{$yellow}Target directory does not exist{/$}");
            bunt::println!("Creating the directory");
            std::fs::create_dir(&user_target).expect("Could not create directory");
        }
        let target_file = PathBuf::from(user_target.clone()).join(name);
        let pb_clone = Arc::clone(&pb);

        // Spawn a new asynchronous task for each file copy operation
        task::spawn(copy_paste(
            pb_clone,
            path.clone(),
            target_file.clone(),
            paste_config.overwrite,
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

            files.iter().for_each(|(_, path)| {
                let redundant_entry = match db::does_exist(conn, path.to_str().unwrap()){
                    Ok(entry) => entry,
                    Err(e) => {
                        bunt::println!("{$red}Failed to check if entry exists: {:?}{/$}\nUse the {$white}-v{/$} flag to see the error", e);
                        std::process::exit(1);
                    }
                };

                db::delete_entry(conn, path.to_str().unwrap()).expect("Unable to delete entry");
            
                if !paste_config.delete{
                    db::insert_into_db(conn, utils::builder_from_entry(&redundant_entry)).expect("Could not insert into database");
                }
            });
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
    overwrite: bool,
) -> Result<(), std::io::Error> {
    tokio::fs::create_dir_all(target.parent().unwrap()).await?;

    let contents = tokio::fs::read(source.clone()).await?;

    if target.exists() && !overwrite {
        bunt::println!(
            "File {$yellow}{}{/$} already exists",
            target.to_str().unwrap()
        );

        bunt::println!("Use the {$green}--overwrite{/$} flag to overwrite the any and all files");
        std::process::exit(1);
    }

    tokio::fs::write(target.clone(), contents.clone()).await?;

    let pb = pb.lock().await;
    pb.inc(1);

    Ok(())
}
