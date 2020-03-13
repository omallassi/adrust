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

    slog::Logger::root(drain, o!())
}

pub fn list_all_adr() -> io::Result<()> {
    let cfg: AdrToolConfig = adr_config::config::get_config();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![b -> "Title", b -> "Date", b-> "Status", b -> "File", b -> "Tags"]);
    for entry in adr_core::adr_repo::list_all_adr(Path::new(&cfg.adr_src_dir))? {
        //table.add_row(row![entry.title, Fg->entry.status, entry.path, entry.tags]);
        let style = match entry.status {
            Status::WIP => "Fy",
            Status::DECIDED => "Fg",
            Status::COMPLETED => "Fg",
            Status::COMPLETES => "Fg",
            _ => "Fr",
        };
        table.add_row(Row::new(vec![
            Cell::new(&entry.title),
            Cell::new(&entry.date),
            Cell::new(&entry.status.as_str()).style_spec(style),
            Cell::new(&entry.path()),
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
    //table.add_row(row![adr_config::config::ADR_ROOT_DIR, cfg.adr_root_dir, "Y"]);
    table.add_row(row![adr_config::config::ADR_SRC_DIR, cfg.adr_src_dir, "Y"]);
    table.add_row(row![adr_config::config::ADR_TEMPLATE_DIR, cfg.adr_template_dir, "Y"]);
    table.add_row(row![adr_config::config::ADR_TEMPLATE_FILE, cfg.adr_template_file, "Y"]);
    table.add_row(row![adr_config::config::ADR_SEARCH_INDEX, cfg.adr_search_index, "N"]);
    table.add_row(row![adr_config::config::LOG_LEVEL, cfg.log_level, "Y"]);
    table.add_row(row![adr_config::config::USE_ID_PREFIX, cfg.use_id_prefix, "Y"]);
    table.add_row(row![adr_config::config::ID_PREFIX_WIDTH, cfg.id_prefix_width, "Y"]);

    // Print the table to stdout
    table.printstd();

    Ok(())
}

fn list_all_tags() -> Result<()> {
    let cfg: AdrToolConfig = adr_config::config::get_config();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![b -> "Tags", b -> "Popularity"]);

    let popularity = adr_core::adr_repo::get_tags_popularity(Path::new(&cfg.adr_src_dir))?;

    for (key, val) in popularity.iter() {
        table.add_row(row![key, val]);
    }

    // Print the table to stdout
    table.printstd();

    Ok(())
}

fn build_index() -> Result<()> {
    let cfg: AdrToolConfig = adr_config::config::get_config();
    let adrs = match adr_core::adr_repo::list_all_adr(Path::new(&cfg.adr_src_dir)) {
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
    table.set_titles(row![b -> "Title", b -> "File", b -> "(Indexed) Tags"]);

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

///
/// The main program - start the CLI ...
fn main() {
    //
    let _options = App::new("adr")
        .version("0.1.0")
        .about("A CLI to help you manage your ADR in git")
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
            App::new("lf")
                .about("Manages ADRs lifecycle")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("new")
                        .about("Creates a new Decision Record")
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
                            Arg::with_name("path")
                                .short("p")
                                .long("path")
                                .takes_value(true)
                                .required(true)
                                .help("Give the path of your Decision Record"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("superseded-by")
                    .about("Supersede the decision with another decision")
                    .version("0.1.0")
                    .arg(
                        Arg::with_name("path")
                            .short("p")
                            .long("path")
                            .takes_value(true)
                            .required(true)
                            .help("Give the path of your Decision Record"),
                    )
                    .arg(
                        Arg::with_name("by")
                            .short("b")
                            .long("by")
                            .takes_value(true)
                            .required(true)
                            .help("Give the path of your Decision Record"),
                    ),
                )
                .subcommand(
                    SubCommand::with_name("completed-by")
                        .about("Complete a decision with another decision")
                        .version("0.1.0")
                        .arg(
                            Arg::with_name("path")
                                .short("p")
                                .long("path")
                                .takes_value(true)
                                .required(true)
                                .help("Give the path of the DR which is completed by"),
                        )
                        .arg(
                            Arg::with_name("by")
                                .short("b")
                                .long("by")
                                .takes_value(true)
                                .required(true)
                                .help("Give the path of the DR which completes"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("obsoleted")
                        .about("Uodate the Status to Obsoleted")
                        .version("0.1.0")
                        .arg(
                            Arg::with_name("path")
                                .short("p")
                                .long("path")
                                .takes_value(true)
                                .required(true)
                                .help("Give the path of your Decision Record"),
                        ),
                )
        )
        .subcommand(
            SubCommand::with_name("search")
                .about("Search across all ADRs")
                .version("0.1.0")
                .args(&[
                    Arg::with_name("query")
                        .short("q")
                        .long("query")
                        .takes_value(true)
                        .required(true)
                        .conflicts_with("build-index")
                        .help("Provide your search query"),
                    Arg::with_name("build-index")
                    .short("b")
                    .long("build-index")
                    .takes_value(false)
                    .required(true)
                    .conflicts_with("query")
                    .help("Build the index based on available ADRs.")
                    ])
            )
        .get_matches();

    //
    match _options.subcommand() {
        ("list", Some(_matches)) => {
            list_all_adr().unwrap();
        }
        ("init", Some(_matches)) => {
            init().unwrap();
        }
        ("lf", Some(matches)) => match matches.subcommand() {
            ("new", Some(matches)) => {
                if matches.is_present("name") {
                    adr_core::adr_repo::create_adr(adr_config::config::get_config(), matches.value_of("name").unwrap()).unwrap();
                }
            }
            ("decided", Some(set_matches)) => {
                if set_matches.is_present("path") {
                    let file_path = set_matches.value_of("path").unwrap();
                    let cfg: AdrToolConfig = adr_config::config::get_config();
                    let base_path = Path::new(&cfg.adr_src_dir);

                    adr_core::adr_repo::transition_to_decided(base_path, file_path).unwrap();
                }
            }
            ("completed-by", Some(set_matches)) => {
                if set_matches.is_present("path") && set_matches.is_present("by") {
                    let cfg: AdrToolConfig = adr_config::config::get_config();
                    let base_path = Path::new(&cfg.adr_src_dir);
                    let file_path = set_matches.value_of("path").unwrap();
                    let by_path = set_matches.value_of("by").unwrap();

                    adr_core::adr_repo::transition_to_completed_by(base_path, file_path, by_path).unwrap();
                }
            }
            ("superseded-by", Some(set_matches)) => {
                if set_matches.is_present("path") && set_matches.is_present("by") {

                    let cfg: AdrToolConfig = adr_config::config::get_config();
                    let base_path = Path::new(&cfg.adr_src_dir);
                    let file_path = set_matches.value_of("path").unwrap();
                    let by_path = set_matches.value_of("by").unwrap();

                    adr_core::adr_repo::transition_to_superseded_by(base_path, file_path, by_path).unwrap();
                }
            }
            ("obsoleted", Some(set_matches)) => {
                if set_matches.is_present("path") {
                    let cfg: AdrToolConfig = adr_config::config::get_config();
                    let base_path = Path::new(&cfg.adr_src_dir);
                    let file_path = set_matches.value_of("path").unwrap();

                    adr_core::adr_repo::transition_to_obsoleted(base_path, file_path).unwrap();
                }
            }
            
            _ => unreachable!(),
        },
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
            if search_matches.is_present("query")  {
                let query = search_matches.value_of("query").unwrap().to_string();
                search(query).unwrap();
            }
            if search_matches.is_present("build-index")  {
                build_index().unwrap();
            }
        },

        ("", None) => println!("No subcommand was used"), // If no subcommand was usd it'll match the tuple ("", None)
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachabe!()
    }
}