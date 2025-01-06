use config::{get_config_from_file, write_default_config, ConstructedArgs};
use files::get_config_path;
use utils::{check_version, print_splash_screen, setup_cli};

mod config;
mod db;
mod files;
mod handler;
mod utils;

#[tokio::main]
async fn main() {
    let mut cmd = setup_cli();
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
