use clap::{command, Arg, ArgAction, Command};
use config::{get_config_from_file, write_default_config, ConstructedArgs};
use files::get_config_path;
use utils::{check_version, print_splash_screen};

mod config;
mod db;
mod files;
mod handler;
mod utils;

#[tokio::main]
async fn main() {
    let mut cmd = command!()
        .author("NoobScience <noobscience@duck.com>")
        .about("Copy paste files in the terminal")
        .arg(
            Arg::new("noignore")
                .short('n')
                .help("Don't respect the .gitignore")
                .long("noignore")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("yes")
                .short('y')
                .help("Prompt yes to all prompts")
                .long("yes")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("all")
                .short('a')
                .long("all")
                .help("Also include hidden files in discovery")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("list").arg(
                Arg::new("size")
                    .id("size")
                    .long("size")
                    .short('s')
                    .help("Calculate and show the size column")
                    .action(ArgAction::SetTrue),
            ).long_about("List the entires in the store"),
        )
        .subcommand(
            Command::new("add")
                .arg(
                    Arg::new("dir")
                        .help("Set sign as a dir")
                        .long("dir")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("preserve-structure")
                        .long("preserve")
                        .help("Preserves the dir structure")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("files")
                        .help("The list of files to add")
                        .num_args(1..)
                        .value_name("FILES"),
                ).long_about("Add entries to the store"),
        )
        .subcommand(
            Command::new("delete").long_about("Delete entries from the ynk store").arg(
                Arg::new("queries")
                    .help("The queries to file the entries")
                    .num_args(1..)
                    .value_name("QUERIES"),
            ),
        )
        .subcommand(
            Command::new("pop")
                .long_about("Pop the last entry in the ynk store")
                .arg(
                    Arg::new("overwrite")
                        .long("overwrite")
                        .help("Overwrite existing files")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("The output dir or file")
                        .num_args(1)
                )
                .arg(Arg::new("strict").help("Error on any IO error").long("strict").action(ArgAction::SetTrue)),
        )
        .subcommand(Command::new("clear").long_about("Clear all entries from the ynk store"))
        .subcommand(
            Command::new("paste")
                .long_about("Paste entries from the ynk store")
                .arg(
                    Arg::new("overwrite")
                        .long("overwrite")
                        .help("Overwrite existing files")
                        .action(ArgAction::SetTrue),
                )
                .arg(Arg::new("strict").help("Error on any IO error").long("strict").action(ArgAction::SetTrue))
                .arg(
                    Arg::new("delete")
                        .long("delete")
                        .help("Delete the entry from the store after pasting")
                        .short('d')
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("The output dir or file")
                        .num_args(1)
                )
                .arg(
                    Arg::new("queries")
                        .help("Queries to filter the entires")
                        .num_args(1..)
                        .value_name("QUERIES")
                )
                .arg(
                    Arg::new("range").long("range").help("Specify the range of entries to paste: Works using the syntax of n..[m]").short('r').num_args(1)
                ),
        ).subcommand(Command::new("completions")
                .arg(
                    Arg::new("shell")
                        .help("The list of files to add")
                        .num_args(1)
                        .value_name("SHELL")
                        .required(true)
                ).long_about("Generate and write completions")
        );

    let matches = cmd.clone().get_matches();

    if !get_config_path().exists() {
        write_default_config();
    }

    let config = get_config_from_file();

    if config.show_splash && atty::is(atty::Stream::Stdout) {
        print_splash_screen();
    }

    files::check_paths_exist();

    let conn = db::connect_to_db().expect("Could not connect to database");

    db::prep_db(&conn).expect("Could not prepare database");

    check_version();

    let mut args = ConstructedArgs::new(config);

    if matches.get_flag("all") {
        args.all = true;
    }
    if matches.get_flag("noignore") {
        args.ignore = false;
    }
    if matches.get_flag("yes") {
        args.yes = false;
    }

    match matches.subcommand_name() {
        Some("list") => {
            let m = matches.subcommand_matches("list").unwrap();
            if m.get_flag("size") {
                args.calculate_size = true;
            }
            handler::handle_list(args, &conn).await;
        }
        Some("add") => {
            let m = matches.subcommand_matches("add").unwrap();
            if m.get_flag("dir") {
                args.dir = true;
            }
            if m.get_flag("preserve-structure") {
                args.preserve_structure = true;
            }
            if let Some(files) = m.get_many::<String>("files") {
                args.files = Some(files.map(|s| s.to_string()).collect::<Vec<String>>());
            }
            handler::handle_add(args, &conn).await;
        }
        Some("pop") => {
            let m = matches.subcommand_matches("pop").unwrap();
            if m.get_flag("overwrite") {
                args.overwrite = true;
            }
            if m.get_flag("strict") {
                args.strict = true;
            }
            let mut output = None;
            if let Some(out) = m.get_one::<String>("output") {
                output = Some(out.clone());
            }
            handler::handle_pop(args, &conn, output).await;
        }
        Some("clear") => {
            handler::handle_clear(args, &conn).await;
        }
        Some("paste") => {
            let m = matches.subcommand_matches("paste").unwrap();
            if m.get_flag("overwrite") {
                args.overwrite = true;
            }
            if m.get_flag("strict") {
                args.strict = true;
            }

            if m.get_flag("delete") {
                args.delete = true;
            }
            if let Some(range) = m.get_one::<String>("range") {
                args.range = Some(range.clone());
            }
            if let Some(files) = m.get_many::<String>("queries") {
                args.files = Some(files.map(|s| s.to_string()).collect::<Vec<String>>());
            }
            let mut output = None;
            if let Some(out) = m.get_one::<String>("output") {
                output = Some(out.clone());
            }
            args.specific = None;

            handler::handle_paste(args, &conn, output).await;
        }
        Some("completions") => {
            let m = matches.subcommand_matches("completions").unwrap();
            let mut c = cmd.clone();
            if let Some(shell) = m.get_one::<String>("shell") {
                handler::handle_completions(&mut c, shell.to_string());
            }
        }
        Some("delete") => {
            let m = matches.subcommand_matches("delete").unwrap();
            if let Some(files) = m.get_many::<String>("queries") {
                args.files = Some(files.map(|s| s.to_string()).collect::<Vec<String>>());
            }

            handler::handle_delete(args, &conn).await;
        }
        Some(_) => {}
        None => {
            let _ = cmd.print_help();
        }
    }
}
