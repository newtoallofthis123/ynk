use std::{
    path::PathBuf, sync::{Arc, OnceLock}
};

use hashbrown::HashMap;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::{sync::Mutex, task};

use crate::{
    db, utils::{self, does_file_exist, list_dir, ListDirConfig, parse_range}, Command, ConstructedArgs
};

/// The main handler function that handles all the commands
/// and the arguments
/// 
/// It sort of acts like a router
/// sending the commands to their respective handlers
/// 
/// # Arguments
/// 
/// * `cmd` - The command to be handled
/// * `args` - The arguments to be handled
/// * `conn` - The database connection
/// 
/// # Note
/// 
/// This function is async because the paste command is async.
/// 
/// # Panics
/// 
/// This function panics if the database connection is not valid
/// and also if at any point, an error occurs while handling the
/// paste command
pub async fn handler(cmd: Command, args: ConstructedArgs, conn: &rusqlite::Connection) {
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
        let mut paste_config = args;
        paste_config.specific = None;

        handle_paste(paste_config, conn).await;
    } else if cmd == Command::Exit {
        bunt::println!("{$yellow}Bye!{/$}");
        std::process::exit(0);
    } else if cmd == Command::List {
        let entries = db::get_all(conn).expect("Could not get entries from database");

        bunt::println!("{$green}{}{/$} files in store", entries.len());
        let mut count = 0;

        entries.iter().for_each(|x| {
            bunt::println!("{}. {$blue}{}{/$}", count, x.path);
            count += 1;
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

        let mut paste_config = args;
        paste_config.range = None;
        paste_config.specific = Some(entry.path);

        handle_paste(paste_config, conn).await
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

        choices.insert("Proceed".to_string(), PathBuf::from("_______"));

        let mut to_delete = Vec::new();
        let mut delete_choices = choices.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>();

        loop {
            let choice = inquire::Select::new(
                "Select a file to delete",
                delete_choices.clone()
            )
            .prompt()
            .unwrap();

            if choice == "Proceed" {
                break;
            }

            to_delete.push(choices.get(&choice).unwrap().clone());
            delete_choices.remove(delete_choices.iter().position(|x| x == &choice).unwrap());
        }

        to_delete.iter().for_each(|x| {
            db::delete_entry(conn, x.to_str().unwrap()).expect("Unable to delete entry");
        });

        bunt::println!("Deleted {$green}{}{/$} files", to_delete.len());
    }
}

/// Private async function to handle the paste command
async fn handle_paste(paste_config: ConstructedArgs, conn: &rusqlite::Connection) {
    let s_files = db::get_all(conn)
        .expect("Could not get entries from database")
        .iter()
        .map(utils::wrap_from_entry)
        .filter(|(_, path)| {
            if paste_config.specific.is_some() {
                return path.to_str().unwrap() == paste_config.specific.as_ref().unwrap();
            }
            true
        })
        .collect::<HashMap<_, _>>();

    let files = if paste_config.range.is_some(){
        let range = paste_config.range.unwrap();
        if range.contains(':'){
            let range_no = range.split(':').collect::<Vec<&str>>();
            if range_no.len() != 2 || range_no.len() > s_files.len(){
                bunt::println!("{$red}Invalid range{/$}");
                std::process::exit(1);
            } else if range_no[0].parse::<usize>().is_err() || range_no[1].parse::<usize>().is_err(){
                bunt::println!("{$red}Invalid range{/$}");
                std::process::exit(1);
            }
        let (start, end) = parse_range(&range);
        s_files
            .iter()
            .enumerate()
            .filter(|(i, _)| *i >= start && *i <= end)
            .map(|(_, (n, p))| (n.clone(), p.clone()))
            .collect::<HashMap<_, _>>()
        } else{
            let index = range.parse::<usize>().unwrap();
            if index > s_files.len(){
                bunt::println!("{$red}Invalid range{/$}");
                std::process::exit(1);
            }
            s_files
                .iter()
                .enumerate()
                .filter(|(i, _)| *i == index)
                .map(|(_, (n, p))| (n.clone(), p.clone()))
                .collect::<HashMap<_, _>>()
        }
    } else{
        s_files
    };

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
            let entries = list_dir(path.to_str().unwrap(), LIST_DIR_CONFIG.get().unwrap());
            final_files.extend(entries.iter().map(|x| {
                let (name, path) = utils::wrap_from_path(path, x);
                (name, path)
            }));
        } else {
            final_files.insert(name.clone(), path.clone());
        }
    });

    let pb = Arc::new(Mutex::new(ProgressBar::new(final_files.len() as u64).with_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    )));

    if utils::is_git_repo(
        &user_target
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
            pb.finish_with_message(format!("Pasted {} files in {}", count, pb.elapsed().as_secs_f32()));

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

    let contents = tokio::fs::read(source).await?;

    if target.exists() && !overwrite {
        bunt::println!(
            "File {$yellow}{}{/$} already exists",
            target.to_str().unwrap()
        );

        bunt::println!("Use the {$green}--overwrite{/$} flag to overwrite the any and all files");
        std::process::exit(1);
    }

    tokio::fs::write(target, contents).await?;

    let pb = pb.lock().await;
    pb.inc(1);

    Ok(())
}
