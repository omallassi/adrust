extern crate slog;
extern crate slog_term;
use slog::*;

use std::io::{self};
use std::path::Path;

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::*;
use comfy_table::{Width::*, CellAlignment, ColumnConstraint::*, Table};

extern crate lazy_static;

extern crate clap;
use clap::{Arg, Command};

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
    table.load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    //table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_header(
        vec!["ID", "Title", "Status", "Date", "File", "Tags"],
    );

    let mut tags_column = table.column_mut(5).expect("This should be the Tags column");
    tags_column.set_constraint(UpperBoundary(Fixed(20)));

    let mut title_column = table.column_mut(1).expect("This should be the Title column");
    title_column.set_constraint(UpperBoundary(Fixed(90)));

    info!(get_logger(), "list all ADR from [{}]",&cfg.adr_src_dir);
    for entry in adr_core::adr_repo::list_all_adr(Path::new(&cfg.adr_src_dir))? {
        //table.add_row(row![entry.title, Fg->entry.status, entry.path, entry.tags]);
        let style = get_cell_style(entry.status);
        table.add_row(
            vec![
                Cell::new(&entry.file_id.to_string()),
                Cell::new(&entry.title).fg(style),
                Cell::new(entry.status.as_str()).fg(style),
                Cell::new(&entry.date),
                Cell::new(&entry.path()),
                Cell::new(&entry.tags).add_attributes(vec![Attribute::Italic]),
            ]
        );
    }

    // Print the table to stdout
    println!("{table}");

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
    table.load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    //table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_header(vec!["Property", "Value", "Modifiable"]);
    //table.add_row(row![adr_config::config::ADR_ROOT_DIR, cfg.adr_root_dir, "Y"]);
    table.add_row(vec![adr_config::config::ADR_SRC_DIR, cfg.adr_src_dir.as_str(), "Y"]);
    table.add_row(vec![
        adr_config::config::ADR_TEMPLATE_DIR,
        cfg.adr_template_dir.as_str(),
        "Y"
    ]);
    table.add_row(vec![
        adr_config::config::ADR_TEMPLATE_FILE,
        cfg.adr_template_file.as_str(),
        "Y"
    ]);
    table.add_row(vec![
        adr_config::config::ADR_SEARCH_INDEX,
        cfg.adr_search_index.as_str(),
        "N"
    ]);
    table.add_row(vec![adr_config::config::LOG_LEVEL, cfg.log_level.to_string().as_str(), "Y"]);
    table.add_row(vec![
        adr_config::config::USE_ID_PREFIX,
        cfg.use_id_prefix.to_string().as_str(),
        "Y"
    ]);
    table.add_row(vec![
        adr_config::config::ID_PREFIX_WIDTH,
        cfg.id_prefix_width.to_string().as_str(),
        "Y"
    ]);

    // Print the table to stdout
    println!("{table}");

    Ok(())
}

fn list_all_tags() -> Result<()> {
    let cfg: AdrToolConfig = adr_config::config::get_config();

    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    //table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_header(vec!["Tags", "Popularity"]);

    let popularity = adr_core::adr_repo::get_tags_popularity(Path::new(&cfg.adr_src_dir))?;

    for (key, val) in popularity.iter() {
        table.add_row(vec![Cell::new(key), Cell::new(&val.to_string())]);
    }

    // Print the table to stdout
    println!("{table}");

    Ok(())
}

fn build_index() -> Result<()> {
    let cfg: AdrToolConfig = adr_config::config::get_config();
    let adrs = match adr_core::adr_repo::list_all_adr(Path::new(&cfg.adr_src_dir)) {
        Ok(e) => e,
        Err(why) => panic!("{:?}", why),
    };
    adr_search::search::build_index(cfg.adr_search_index, adrs).unwrap();

    Ok(())
}

fn search(query: String) -> Result<()> {
    let cfg: AdrToolConfig = adr_config::config::get_config();

    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    //table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_header(vec!["Title", "Status", "Date", "File", "(Indexed) Tags"]);

    let mut tags_column = table.column_mut(4).expect("This should be the Tags column");
    tags_column.set_constraint(UpperBoundary(Fixed(20)));

    let mut title_column = table.column_mut(0).expect("This should be the Title column");
    title_column.set_constraint(UpperBoundary(Fixed(90)));

    //TODO get limit value from AdrToolConfig
    let limit: usize = 100;

    let results = match adr_search::search::search(cfg.adr_search_index, query, limit) {
        Ok(e) => e,
        Err(why) => panic!("{:?}", why),
    };
    let results_size = &results.len();

    for entry in results {
        let status = &entry.status[0];
        let status_as_enum = Status::from_str(String::from(status));
        let style = get_cell_style(status_as_enum);

        table.add_row(
            vec![
                Cell::new(&entry.title[0]).fg(style),
                Cell::new(&entry.status[0]).fg(style),
                Cell::new(&entry.date[0]),
                Cell::new(&entry.path[0]),
                Cell::new(&entry.tags[0]).add_attributes(vec![Attribute::Italic]),
            ]
        );
    }

    println!("{table}");

    println!("\n Displayed {:?} results - Results are limited to {:?} items - run adr config -h to change configuration", &results_size, &limit);

    Ok(())
}

fn get_cell_style(status: Status) -> Color {
    let style = match status {

        Status::WIP => Color::DarkYellow,
        Status::DECIDED => Color::DarkGreen,
        Status::COMPLETED => Color::Green,
        Status::COMPLETES => Color::Green,
        _ => Color::DarkRed,
    };

    style
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
    let cmd = Command::new("adr")
        .version("0.1.0")
        .about("A CLI to help you manage your ADR in git")
        .subcommand(
            Command::new("list")
                .about("Lists all Decision Records")
                .version("0.1.0"),
        )
        .subcommand(
            Command::new("init")
                .about("Init ADRust based on config")
                .version("0.1.0"),
        )
        .subcommand(
            Command::new("config")
                .about("Manage Configuration Items")
                .subcommand_required(true)
                .subcommand(
                    Command::new("set")
                        .about("Update Configuration Item with specified value")
                        .arg(
                            Arg::new("name")
                                .short('n')
                                .long("name")
                                .required(true)
                                .takes_value(true)
                                .help("the name of the property"),
                        )
                        .arg(
                            Arg::new("value")
                                .short('v')
                                .long("value")
                                .required(true)
                                .takes_value(true)
                                .help("the value of the property"),
                        ),
                )
                .subcommand(
                    Command::new("list").about("List All the Configuration Items"),
                ),
        )
        .subcommand(
            Command::new("tags")
                .about("Manage Tags")
                .subcommand_required(true)
                .subcommand(Command::new("list").about("List All the Tags")),
        )
        .subcommand(
            Command::new("lf")
                .about("Manages ADRs lifecycle")
                .subcommand_required(true)
                .subcommand(
                    Command::new("new")
                        .about("Creates a new Decision Record")
                        .version("0.1.0")
                        .arg(
                            Arg::new("title")
                                .short('t')
                                .long("title")
                                .takes_value(true)
                                .required(true)
                                .help("Give the title of your Decision Record"),
                        )
                        .arg(
                            Arg::new("path")
                                .short('p')
                                .long("path")
                                .takes_value(true)
                                .required(false)
                                .help("Specify relative path (nested directories)"),
                        ),
                )
                .subcommand(
                    Command::new("decided")
                        .about("update the Status to Decide")
                        .version("0.1.0")
                        .arg(
                            Arg::new("path")
                                .short('p')
                                .long("path")
                                .takes_value(true)
                                .required(true)
                                .help("Give the path of your Decision Record"),
                        ),
                )
                .subcommand(
                    Command::new("superseded-by")
                        .about("Supersede the decision with another decision")
                        .version("0.1.0")
                        .arg(
                            Arg::new("path")
                                .short('p')
                                .long("path")
                                .takes_value(true)
                                .required(true)
                                .help("Give the path of your Decision Record"),
                        )
                        .arg(
                            Arg::new("by")
                                .short('b')
                                .long("by")
                                .takes_value(true)
                                .required(true)
                                .help("Give the path of your Decision Record"),
                        ),
                )
                .subcommand(
                    Command::new("completed-by")
                        .about("Complete a decision with another decision")
                        .version("0.1.0")
                        .arg(
                            Arg::new("path")
                                .short('p')
                                .long("path")
                                .takes_value(true)
                                .required(true)
                                .help("Give the path of the DR which is completed by"),
                        )
                        .arg(
                            Arg::new("by")
                                .short('b')
                                .long("by")
                                .takes_value(true)
                                .required(true)
                                .help("Give the path of the DR which completes"),
                        ),
                )
                .subcommand(
                    Command::new("obsoleted")
                        .about("Uodate the Status to Obsoleted")
                        .version("0.1.0")
                        .arg(
                            Arg::new("path")
                                .short('p')
                                .long("path")
                                .takes_value(true)
                                .required(true)
                                .help("Give the path of your Decision Record"),
                        ),
                ),
        )
        .subcommand(
            Command::new("search")
                .about("Search across all ADRs")
                .version("0.1.0")
                .args(&[
                    Arg::new("query")
                        .short('q')
                        .long("query")
                        .takes_value(true)
                        .required(true)
                        .conflicts_with_all(&["build-index", "title"])
                        .help("Provide your search query. The following syntax can be used :\n\
                            \ta AND b OR c will search for documents containing terms (a and b) or c, \n\
                            \t-b will search documents that do not contain the term b, \n\
                            \t+c will search documents that must contain the term c, \n\
                            \ttags:a AND tags:b will search for documents that have the tags a and b, \n\
                            \ttitle:a will search on title of the document, \n\
                            \tdate:[2022-08-01T00:00:00Z TO 2023-10-02T18:00:00Z] AND tags:BPaaS will search between the specified range and date (and specified tag), \n\
                            \tstatus:decided will search for decided documents"),
                    Arg::new("build-index")
                        .short('b')
                        .long("build-index")
                        .takes_value(false)
                        .required(true)
                        .conflicts_with_all(&["query", "title"])
                        .help("Build the index based on available ADRs."),
                    Arg::new("title")
                        .short('t')
                        .long("title")
                        .takes_value(true)
                        .required(true)
                        .conflicts_with_all(&["build-index", "query"])
                        .help("Search on title property of ADR only"),
                ]),
        );

    //
    let _matches = cmd.get_matches();
    let subcommand = _matches.subcommand();


    match subcommand {
        Some(("list", _matches)) => {
            list_all_adr().unwrap();
        }
        Some(("init", _matches)) => {
            init().unwrap();
        }
        Some(("lf", matches)) => match matches.subcommand() {
            Some(("new", matches)) => {
                if matches.is_present("title") {
                    adr_core::adr_repo::create_adr(
                        adr_config::config::get_config(),
                        matches.value_of("path"),
                        matches.value_of("title").unwrap(),
                    )
                    .unwrap();
                }
            }
            Some(("decided", set_matches)) => {
                if set_matches.is_present("path") {
                    let file_path = set_matches.value_of("path").unwrap();
                    let cfg: AdrToolConfig = adr_config::config::get_config();
                    let base_path = Path::new(&cfg.adr_src_dir);

                    adr_core::adr_repo::transition_to_decided(base_path, file_path).unwrap();
                }
            }
            Some(("completed-by", set_matches)) => {
                if set_matches.is_present("path") && set_matches.is_present("by") {
                    let cfg: AdrToolConfig = adr_config::config::get_config();
                    let base_path = Path::new(&cfg.adr_src_dir);
                    let file_path = set_matches.value_of("path").unwrap();
                    let by_path = set_matches.value_of("by").unwrap();

                    adr_core::adr_repo::transition_to_completed_by(base_path, file_path, by_path)
                        .unwrap();
                }
            }
            Some(("superseded-by", set_matches)) => {
                if set_matches.is_present("path") && set_matches.is_present("by") {
                    let cfg: AdrToolConfig = adr_config::config::get_config();
                    let base_path = Path::new(&cfg.adr_src_dir);
                    let file_path = set_matches.value_of("path").unwrap();
                    let by_path = set_matches.value_of("by").unwrap();

                    adr_core::adr_repo::transition_to_superseded_by(base_path, file_path, by_path)
                        .unwrap();
                }
            }
            Some(("obsoleted", set_matches)) => {
                if set_matches.is_present("path") {
                    let cfg: AdrToolConfig = adr_config::config::get_config();
                    let base_path = Path::new(&cfg.adr_src_dir);
                    let file_path = set_matches.value_of("path").unwrap();

                    adr_core::adr_repo::transition_to_obsoleted(base_path, file_path).unwrap();
                }
            }

            _ => unreachable!(),
        },
        Some(("config", config_matches)) => match config_matches.subcommand() {
            Some(("list", _remote_matches)) => {
                list_all_config().unwrap();
            }
            Some(("set", set_matches)) => {
                set_config(
                    set_matches.value_of("name").unwrap(),
                    set_matches.value_of("value").unwrap(),
                )
                .unwrap();
            }
            _ => unreachable!(),
        },
        Some(("tags", tags_matches)) => match tags_matches.subcommand() {
            Some(("list", _remote_matches)) => {
                list_all_tags().unwrap();
            }
            _ => unreachable!(),
        },
        Some(("search", search_matches)) => {
            if search_matches.is_present("query") {
                let query = search_matches.value_of("query").unwrap().to_string();
                search(query).unwrap();
            }
            if search_matches.is_present("build-index") {
                build_index().unwrap();
            }
            if search_matches.is_present("title") {
                let query = search_matches.value_of("title").unwrap().to_string();
                search("title:".to_string() + &query).unwrap();
            }
        }

        _ => println!("Please, try adr --help"), // If all subcommands are defined above, anything else is unreachabe!()
    }
}
