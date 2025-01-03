use clap::{Arg, ArgAction, Command};
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
    let cmd = Command::new("ynk")
        .author("NoobScience <noobscience@duck.com>")
        .about("Copy paste files in the terminal")
        .arg(
            Arg::new("noignore")
                .short('n')
                .long("noignore")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("yes")
                .short('y')
                .long("yes")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("all")
                .short('a')
                .long("all")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("list").arg(
                Arg::new("size")
                    .id("size")
                    .long("size")
                    .help("Show the size as well")
                    .action(ArgAction::SetTrue),
            ),
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
                        .help("The list of files")
                        .num_args(1..)
                        .value_name("FILES"),
                ),
        )
        .subcommand(
            Command::new("delete").arg(
                Arg::new("files")
                    .help("The list of files")
                    .num_args(1..)
                    .value_name("FILES"),
            ),
        )
        .subcommand(
            Command::new("pop")
                .arg(
                    Arg::new("overwrite")
                        .long("overwrite")
                        .action(ArgAction::SetTrue),
                )
                .arg(Arg::new("strict").long("strict").action(ArgAction::SetTrue)),
        )
        .subcommand(Command::new("clear"))
        .subcommand(
            Command::new("paste")
                .arg(
                    Arg::new("overwrite")
                        .long("overwrite")
                        .action(ArgAction::SetTrue),
                )
                .arg(Arg::new("strict").long("strict").action(ArgAction::SetTrue))
                .arg(
                    Arg::new("delete")
                        .long("delete")
                        .short('d')
                        .action(ArgAction::SetTrue),
                )
                .arg(Arg::new("range").long("range").short('r').num_args(1)),
        );

    let matches = cmd.clone().get_matches();

    if !get_config_path().exists() {
        write_default_config();
    }

    let config = get_config_from_file();

    if config.show_splash {
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
            handler::handle_pop(args, &conn).await;
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
            args.specific = None;

            handler::handle_paste(args, &conn).await;
        }
        Some("delete") => {
            let m = matches.subcommand_matches("delete").unwrap();
            if let Some(files) = m.get_many::<String>("files") {
                args.files = Some(files.map(|s| s.to_string()).collect::<Vec<String>>());
            }

            handler::handle_delete(args, &conn).await;
        }
        Some(_) => {}
        None => todo!(),
    }
}
