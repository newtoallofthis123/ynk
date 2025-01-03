//! The main handler function that handles all the commands
//! this is essentially the router of the program

use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use clap::Command;
use clap_complete::{generate, Shell};
use colored::Colorize;
use hashbrown::HashMap;
use indicatif::{ProgressBar, ProgressStyle};
use tabled::{
    settings::{Panel, Style},
    Table, Tabled,
};
use tokio::{sync::Mutex, task};

use crate::{
    db,
    utils::{self, deep_search, does_file_exist, list_dir, sort_entries, ListDirConfig},
    ConstructedArgs,
};

pub async fn handle_delete(args: ConstructedArgs, conn: &rusqlite::Connection) {
    let mut entries = db::get_all(conn).expect("Could not get entries from database");

    sort_entries(&mut entries);

    if entries.is_empty() {
        println!("No entries in the store");
        std::process::exit(1);
    }

    let choices = entries
        .iter()
        .map(utils::wrap_from_entry)
        .collect::<HashMap<_, _>>();

    let mut to_delete = Vec::new();

    if let Some(queries) = args.files {
        to_delete = deep_search(queries, &entries)
            .iter()
            .map(|e| PathBuf::from(e.path.clone()))
            .collect();
    } else {
        handle_list(args, conn).await;
        println!(
            "{}",
            "Enter the id of the files to delete seperate by a space".yellow()
        );
        let indexes = inquire::Text::new("Enter the indexes or the names")
            .with_placeholder("Ex: 1 README.md 4")
            .prompt()
            .unwrap();

        let indexes = indexes.split_whitespace().collect::<Vec<&str>>();
        indexes.iter().for_each(|x| {
            let index = x.parse::<i32>();
            if let Ok(idx) = index {
                let e = entries.iter().find(|x| x.id == idx);
                if let Some(entry) = e {
                    to_delete.push(PathBuf::from(entry.path.clone()));
                } else {
                    println!("{}", "Invalid index".red());
                    std::process::exit(1);
                }
            } else if let Some(entry) = choices.get(*x) {
                to_delete.push(entry.clone());
            } else {
                println!("{}", "Invalid index".red());
                std::process::exit(1);
            }
        });
    }

    to_delete.iter().for_each(|x| {
        db::delete_entry(conn, x.to_str().unwrap()).expect("Unable to delete entry");
    });

    // Reid all the remaining files
    let _ = db::reid(conn).expect("Failed to reid");
    println!("Deleted {} files", to_delete.len().to_string().green());
}

pub async fn handle_clear(args: ConstructedArgs, conn: &rusqlite::Connection) {
    if args.yes {
        let choice = inquire::Confirm::new("Are you sure you want to clear all the copied files?")
            .with_default(false)
            .prompt()
            .unwrap();

        if !choice {
            println!("Ok! Quitting");
        }
    }

    db::delete_all(conn).expect("Unable to delete the indexes");
    println!("Emptied the store");
}

pub async fn handle_pop(
    args: ConstructedArgs,
    conn: &rusqlite::Connection,
    output: Option<String>,
) {
    let entry = match db::pop_one(conn) {
        Ok(entry) => entry,
        Err(e) => {
            println!("Could not pop entry from database: {:?}", e);
            std::process::exit(1);
        }
    };

    let mut paste_config = args;
    paste_config.range = None;
    paste_config.specific = Some(entry.path);
    paste_config.delete = true;

    handle_paste(paste_config, conn, output).await
}

pub async fn handle_add(args: ConstructedArgs, conn: &rusqlite::Connection) {
    let mut files: HashMap<String, PathBuf> = HashMap::new();
    let req = args.files.unwrap_or_else(|| {
        println!("{}", "No files or directories specified".yellow());
        println!("Copying the current directory");

        if args.yes {
            let choice = inquire::Confirm::new("Do you want to continue?")
                .with_default(true)
                .prompt()
                .unwrap();

            if !choice {
                std::process::exit(0);
            }
        }

        vec![".".to_string()]
    });
    req.iter().for_each(|x| {
        if !does_file_exist(x) {
            println!(
                "{} \"{}\" {}",
                "File or directory with path".red(),
                x.red(),
                "does not exist.".red(),
            );
            std::process::exit(1);
        }

        let path = if args.preserve_structure {
            x.clone()
        } else {
            utils::parse_file_name(x)
        };

        files.insert(path, PathBuf::from(x).canonicalize().unwrap());
    });

    let entries = utils::construct_entry_builders(&files)
        .iter()
        .map(|x| db::insert_into_db(conn, x.to_owned()).expect("Could not insert into database"))
        .collect::<Vec<_>>();

    println!("Copied {} files", entries.len());
}

fn parse_range(range: String, s_files: &[db::Entry]) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    if range.contains("..") {
        let range = range.split("..").collect::<Vec<&str>>();
        let start = range[0].parse::<usize>().unwrap();
        let end = range[1].parse::<usize>().unwrap();
        for i in start..=end {
            let entry = s_files.iter().find(|x| x.id as usize == i).unwrap();
            files.push(utils::wrap_from_entry(entry));
        }
    } else {
        let ids = range
            .split(',')
            .map(|x| x.parse::<usize>().unwrap())
            .collect::<Vec<usize>>();
        for i in ids {
            let entry = s_files.iter().find(|x| x.id as usize == i).unwrap();
            files.push(utils::wrap_from_entry(entry));
        }
    };

    files
}

pub async fn handle_paste(
    paste_config: ConstructedArgs,
    conn: &rusqlite::Connection,
    output: Option<String>,
) {
    let s_files = db::get_all(conn).expect("Could not get entries from database");
    let queries = paste_config.files.unwrap_or_default();
    let s_files = deep_search(queries, &s_files);
    let range = paste_config.range.clone();
    let files = if let Some(range) = range {
        parse_range(range, &s_files)
    } else if let Some(specific_path) = paste_config.specific {
        s_files
            .iter()
            .filter(|e| e.path == specific_path)
            .map(utils::wrap_from_entry)
            .collect()
    } else {
        s_files.iter().map(utils::wrap_from_entry).collect()
    };

    let user_target = output.unwrap_or_else(|| ".".to_string()).clone();

    static LIST_DIR_CONFIG: OnceLock<ListDirConfig> = OnceLock::new();
    LIST_DIR_CONFIG.get_or_init(|| ListDirConfig {
        filter_file: !paste_config.dir,
        full_path: false,
        strict: paste_config.strict,
        hidden: paste_config.all,
        respect_ignore: paste_config.ignore,
    });

    let mut final_files = HashMap::new();
    let mut file_sizes = 0.0;

    files.iter().for_each(|(name, path)| {
        if path.is_dir() {
            let (entries, got_size) =
                list_dir(path.to_str().unwrap(), LIST_DIR_CONFIG.get().unwrap());
            file_sizes += got_size;
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

    let tasks = final_files.iter().map(|(name, path)| {
        if !PathBuf::from(user_target.clone()).exists() {
            println!("{}", "Target directory does not exist".yellow());
            println!("Creating the directory");
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
                    println!(
                        "Failed to paste file: {:?}\nUse the -v flag to see the error",
                        e
                    );
                } else {
                    count += 1
                }
            });

            let pb = pb.lock().await;
            pb.finish_with_message(format!(
                "\nPasted {} files in {} seconds",
                count,
                pb.elapsed().as_secs_f32()
            ));

            println!(
                "Total size of files: {}",
                utils::convert_size(file_sizes).to_string().green()
            );

            files.iter().for_each(|(_, path)| {
                // update access time
                db::update_accessed_at(conn, path.to_str().unwrap())
                    .expect("Could not update access time");

                if paste_config.delete {
                    db::delete_entry(conn, path.to_str().unwrap()).expect("Unable to delete entry");
                }
            });
            if paste_config.delete {
                // Reid all the remaining files
                let _ = db::reid(conn).expect("Failed to reid");
            }
        }
        Err(e) => {
            println!(
                "Failed to paste files: {:?}\nUse the -v flag to see the error",
                e
            );
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
        println!("File {} already exists", target.to_str().unwrap());

        println!("Use the --overwrite flag to overwrite the any and all files");
        std::process::exit(1);
    }

    tokio::fs::write(target, contents).await?;

    let pb = pb.lock().await;
    pb.inc(1);

    Ok(())
}

pub async fn handle_list(args: ConstructedArgs, conn: &rusqlite::Connection) {
    let mut entries = db::get_all(conn).expect("Could not get entries from database");

    sort_entries(&mut entries);

    if entries.is_empty() {
        println!("{}", "No entries in the store".red());
        std::process::exit(1);
    }

    println!(
        "{}  entries in the store",
        entries.len().to_string().green()
    );
    let mut count = 0;

    #[derive(Tabled)]
    struct DisplayFiles {
        id: usize,
        name: String,
        path: String,
        count: usize,
        size: String,
        last_accessed: String,
    }

    #[derive(Tabled)]
    struct PartialDisplayFiles {
        id: usize,
        name: String,
        path: String,
        last_accessed: String,
    }

    let mut paste_config = args;
    paste_config.specific = None;

    static LIST_DIR_CONFIG: OnceLock<ListDirConfig> = OnceLock::new();
    LIST_DIR_CONFIG.get_or_init(|| ListDirConfig {
        filter_file: !paste_config.dir,
        full_path: false,
        strict: paste_config.strict,
        hidden: paste_config.all,
        respect_ignore: paste_config.ignore,
    });

    // TODO: Better way to handle the calculate size flag
    #[allow(unused_assignments)]
    let mut table = String::new();

    let mut total_size = 0.0;
    if paste_config.calculate_size {
        let mut display_contents = Vec::new();
        entries.iter().for_each(|x| {
            let mut file_count = 1;
            let mut size = 0.0;

            utils::convert_size(size);

            if PathBuf::from(x.path.clone()).is_dir() {
                let (files, raw_size) = utils::list_dir(&x.path, LIST_DIR_CONFIG.get().unwrap());

                file_count = files.len();
                size = raw_size;
            } else {
                size = PathBuf::from(x.path.clone()).metadata().unwrap().len() as f64;
            }

            total_size += size;

            display_contents.push(DisplayFiles {
                id: x.id as usize,
                name: x.name.clone(),
                path: x.path.clone(),
                count: file_count,
                size: utils::convert_size(size),
                last_accessed: x.accessed_at.to_rfc2822(),
            });
            count += 1;
        });

        table = Table::new(display_contents)
            .with(Style::modern_rounded())
            .with(Panel::header("Entries in The Store"))
            .to_string();
    } else {
        let mut display_contents = Vec::new();
        entries.iter().for_each(|x| {
            display_contents.push(PartialDisplayFiles {
                id: x.id as usize,
                name: x.name.clone(),
                path: x.path.clone(),
                last_accessed: x.accessed_at.to_rfc2822(),
            });
            count += 1;
        });

        table = Table::new(display_contents)
            .with(Style::modern_rounded())
            .with(Panel::header("Entries in The Store"))
            .to_string();
    }

    println!("{}", table);

    if paste_config.calculate_size {
        println!(
            "Total size of {} kept track",
            utils::convert_size(total_size).green()
        );
    }
    println!("The entry {} can be popped", entries[0].path.blue(),);

    println!("Use ynk paste to paste the files");
}

fn map_to_shell(shell: &str) -> Shell {
    match shell {
        "fish" => Shell::Fish,
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "powershell" => Shell::PowerShell,
        _ => Shell::Bash,
    }
}

pub fn handle_completions(command: &mut Command, shell: String) {
    let sh = map_to_shell(&shell);
    let mut res: Vec<u8> = Vec::new();
    generate(sh, command, command.get_name().to_string(), &mut res);

    let completions = String::from_utf8_lossy(&res).to_string();
    println!("{}", completions);
}
