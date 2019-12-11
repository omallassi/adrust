extern crate slog;
extern crate slog_term;
use slog::*;

use std::io::{self};
use std::path::Path;

#[macro_use]
extern crate prettytable;
use prettytable::format;
use prettytable::{Cell, Row, Table};

extern crate lazy_static;

extern crate clap;
use clap::{App, AppSettings, Arg, SubCommand};

extern crate dirs;

extern crate adr_core;
use adr_core::adr_repo::Status;
extern crate adr_config;
use adr_config::config::AdrToolConfig;
extern crate adr_search;

fn get_logger() -> slog::Logger {
    let cfg: AdrToolConfig = adr_config::config::get_config();

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let drain = slog::LevelFilter::new(
        drain,
        Level::from_usize(cfg.log_level).unwrap_or(Level::Debug),
    )
    .fuse();

    let log = slog::Logger::root(drain, o!());

    log
}

pub fn list_all_adr() -> io::Result<()> {
    let cfg: AdrToolConfig = adr_config::config::get_config();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![b -> "Title", b-> "Status", b -> "File", b -> "Tags"]);
    for entry in adr_core::adr_repo::list_all_adr(&cfg.adr_src_dir)? {
        //table.add_row(row![entry.title, Fg->entry.status, entry.path, entry.tags]);
        let style = match entry.status {
            Status::WIP => "Fy",
            Status::DECIDED => "Fg",
            _ => "Fr",
        };
        table.add_row(Row::new(vec![
            Cell::new(&entry.title),
            Cell::new(&entry.status.as_str()).style_spec(style),
            Cell::new(&entry.path),
            Cell::new(&entry.tags),
        ]));
    }

    // Print the table to stdout
    table.printstd();

    Ok(())
}

fn set_config(name: &str, value: &str) -> Result<()> {
    adr_config::config::set_config(name, value)
}

/**
 * default config will be stored in directories::ProjectDir::config_dir() (a.k.a ls -la $HOME/Library/Preferences/)
 *
 * TODO need to find a proper way to map to the config struct - could be managed with a macro
 */
fn list_all_config() -> Result<()> {
    info!(get_logger(), "list all configuration elements",);
    let cfg: AdrToolConfig = adr_config::config::get_config();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![b -> "Property", b -> "Value", b -> "Modifiable"]);
    table.add_row(row!["adr_root_dir", cfg.adr_root_dir, "Y"]);
    table.add_row(row!["adr_src_dir", cfg.adr_src_dir, "N"]);
    table.add_row(row!["adr_template_dir", cfg.adr_template_dir, "N"]);
    table.add_row(row!["adr_search_dir", cfg.adr_search_index, "N"]);
    table.add_row(row!["log_level", cfg.log_level, "Y"]);

    // Print the table to stdout
    table.printstd();

    Ok(())
}

fn list_all_tags() -> Result<()> {
    let cfg: AdrToolConfig = adr_config::config::get_config();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![b -> "Tags"]);
    for entry in adr_core::adr_repo::list_all_adr(&cfg.adr_src_dir)? {
        table.add_row(row![entry.tags]);
    }

    // Print the table to stdout
    table.printstd();

    Ok(())
}

fn build_index() -> Result<()> {
    let cfg: AdrToolConfig = adr_config::config::get_config();
    let adrs = match adr_core::adr_repo::list_all_adr(&cfg.adr_src_dir) {
        Ok(e) => e,
        Err(why) => panic!(format!("{:?}", why)),
    };
    adr_search::search::build_index(cfg.adr_search_index, adrs).unwrap();    

    Ok(())
}

fn search(query: String) -> Result<()> {
    let cfg: AdrToolConfig = adr_config::config::get_config();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![b -> "Title", b -> "File", b -> "Tags"]);

    let results = match adr_search::search::search(cfg.adr_search_index, query) {
        Ok(e) => e,
        Err(why) => panic!(format!("{:?}", why)),
    };

    for entry in  results{
        table.add_row(Row::new(vec![
            Cell::new(&entry.title[0]),
            Cell::new(&entry.path[0]),
            Cell::new(&entry.tags[0]),
        ]));
    }

    table.printstd();

    Ok(())
}

/**
 * init based on config
 */
fn init() -> Result<()> {
    adr_config::config::init()
}

fn main() {
    let cfg: AdrToolConfig = adr_config::config::get_config();
    //
    let _options = App::new("adr")
        .version("0.1.0")
        .about("A CLI to help you manage your ADR in git")
        .subcommand(
            SubCommand::with_name("new")
                .about("will create a new Decision Record")
                .version("0.1.0")
                .arg(
                    Arg::with_name("name")
                        .short("n")
                        .long("name")
                        .takes_value(true)
                        .required(true)
                        .help("Give the name of your Decision Record"),
                ),
        )
        .subcommand(
            SubCommand::with_name("decided")
                .about("update the Status to Decide")
                .version("0.1.0")
                .arg(
                    Arg::with_name("name")
                        .short("n")
                        .long("name")
                        .takes_value(true)
                        .required(true)
                        .help("Give the name of your Decision Record"),
                ),
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("Lists all Decision Records")
                .version("0.1.0"),
        )
        .subcommand(
            SubCommand::with_name("init")
                .about("Init ADRust based on config")
                .version("0.1.0"),
        )
        .subcommand(
            App::new("config")
                .about("Manage Configuration Items")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("set")
                        .about("Update Configuration Item with specified value")
                        .arg(
                            Arg::with_name("name")
                                .short("n")
                                .long("name")
                                .required(true)
                                .takes_value(true)
                                .help("the name of the property"),
                        )
                        .arg(
                            Arg::with_name("value")
                                .short("v")
                                .long("value")
                                .required(true)
                                .takes_value(true)
                                .help("the value of the property"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("list").about("List All the Configuration Items"),
                ),
        )
        .subcommand(
            App::new("tags")
                .about("Manage Tags")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(SubCommand::with_name("list").about("List All the Tags")),
        )
        .subcommand(
            SubCommand::with_name("superseded-by")
                .about("update the Status to Decide")
                .version("0.1.0")
                .arg(
                    Arg::with_name("name")
                        .short("n")
                        .long("name")
                        .takes_value(true)
                        .required(true)
                        .help("Give the name of your Decision Record"),
                )
                .arg(
                    Arg::with_name("by")
                        .short("b")
                        .long("by")
                        .takes_value(true)
                        .required(true)
                        .help("Give the name of your Decision Record"),
                ),
        )
        .subcommand(
            SubCommand::with_name("completed-by")
                .about("Complete a decision with another decision")
                .version("0.1.0")
                .arg(
                    Arg::with_name("name")
                        .short("n")
                        .long("name")
                        .takes_value(true)
                        .required(true)
                        .help("Give the name of the DR which is completed by"),
                )
                .arg(
                    Arg::with_name("by")
                        .short("b")
                        .long("by")
                        .takes_value(true)
                        .required(true)
                        .help("Give the name of the DR which completes"),
                ),
        )
        .subcommand(
            SubCommand::with_name("search")
                .about("Search across all ADRs")
                .version("0.1.0")
                .arg(
                    Arg::with_name("query")
                        .short("q")
                        .long("query")
                        .takes_value(true)
                        .required(true)
                        .help("Provide your search query"),
                ),
        )
        .subcommand(
            SubCommand::with_name("index")
                .about("Index all available ADRs")
                .version("0.1.0")
                .arg(
                    Arg::with_name("build")
                    .short("build")
                    .long("build")
                    .takes_value(false)
                    .required(true)
                    .help("Build the index based on available ADRs.")
                ),
        )
        .get_matches();

    //
    match _options.subcommand() {
        ("new", Some(matches)) => {
            if matches.is_present("name") {
                adr_core::adr_repo::create_adr(
                    matches.value_of("name").unwrap(),
                    Path::new(&cfg.adr_template_dir),
                    Path::new(&cfg.adr_src_dir),
                )
                .unwrap();
            }
        }
        ("list", Some(_matches)) => {
            list_all_adr().unwrap();
        }
        ("init", Some(_matches)) => {
            init().unwrap();
        }
        ("decided", Some(_matches)) => {
            if _matches.is_present("name") {
                adr_core::adr_repo::update_to_decided(_matches.value_of("name").unwrap()).unwrap();
            }
        }
        ("superseded-by", Some(_matches)) => {
            if _matches.is_present("name") && _matches.is_present("by") {
                adr_core::adr_repo::superseded_by(
                    _matches.value_of("name").unwrap(),
                    _matches.value_of("by").unwrap(),
                )
                .unwrap();
            }
        }
        ("completed-by", Some(_matches)) => {
            if _matches.is_present("name") && _matches.is_present("by") {
                adr_core::adr_repo::completed_by(
                    _matches.value_of("name").unwrap(),
                    _matches.value_of("by").unwrap(),
                )
                .unwrap();
            }
        }
        ("config", Some(config_matches)) => match config_matches.subcommand() {
            ("list", Some(_remote_matches)) => {
                list_all_config().unwrap();
            }
            ("set", Some(set_matches)) => {
                set_config(
                    set_matches.value_of("name").unwrap(),
                    set_matches.value_of("value").unwrap(),
                )
                .unwrap();
            }
            _ => unreachable!(),
        },
        ("tags", Some(tags_matches)) => match tags_matches.subcommand() {
            ("list", Some(_remote_matches)) => {
                list_all_tags().unwrap();
            }
            _ => unreachable!(),
        },
        ("search", Some(search_matches)) => {
            let query = search_matches.value_of("query").unwrap().to_string();
            search(query).unwrap();
        }
        ("index", Some(_matches)) => {
            if _matches.is_present("build")  {
                build_index().unwrap();
            }   
        },

        ("", None) => println!("No subcommand was used"), // If no subcommand was usd it'll match the tuple ("", None)
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachabe!()
    }
}
