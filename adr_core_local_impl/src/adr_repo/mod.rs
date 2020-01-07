extern crate slog;
extern crate slog_term;
use slog::*;

use std::fmt::Write;
use std::fs::{self, File};
use std::io::prelude::*;
use std::io::{self};
use std::path::Path;

extern crate regex;
use regex::Regex;

use walkdir::{DirEntry, WalkDir};

extern crate adr_config;
use adr_config::config::AdrToolConfig;

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

///
/// Creates the file (based on template file). Returns true if file is created, false if not
/// (because target file already exists...)
pub fn create_adr(name: &str, path_to_template: &Path, src_dir: &Path) -> io::Result<bool> {
    let name = match format_decision_name(name) {
        Ok(name) => name,
        Err(_why) => panic!(format!("Problem while formatting name [{}]", name)),
    };
    let target_path = src_dir.join(format!("{}.adoc", name));
    let is_target_file = target_path.is_file();
    if !is_target_file {
        match path_to_template.exists() {
            true => {
                fs::copy(path_to_template, &target_path)?;
                info!(get_logger(), "New ADR {:?} created", target_path);
            }
            false => {
                error!(get_logger(), "[{}] was not found", path_to_template.to_string_lossy());
            }
        }
    } else {
        error!(
            get_logger(),
            "Decision already exists. Please use another name",
        );
    }

    Ok(!is_target_file)
}

fn extract_seq_id(name: &str) -> Result<usize> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(\d+)").unwrap();
    }

    let cap = match RE.captures(name) {
        Some(val) => val,
        None => {
            error!(get_logger(), "Unable to extract_seq_id from [{}]", name);
            panic!();
        }
    };

    debug!(get_logger(), "found first match [{}]", cap[0].to_string());
    let id: usize = cap[0].to_string().parse().unwrap();

    Ok(id)
}

pub fn format_decision_name(name: &str) -> Result<String> {
    let name = name.to_ascii_lowercase();
    let name = name.replace(" ", "-");

    Ok(name.to_string())
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

#[derive(Debug, PartialEq)]
pub enum Status {
    WIP,
    DECIDED,
    COMPLETED,
    COMPLETES,
    SUPERSEDED,
    SUPERSEDES,
    OBSOLETED,
    NONE,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match *self {
            Status::WIP => "wip",
            Status::DECIDED => "decided",
            Status::COMPLETED => "completed",
            Status::COMPLETES => "completes",
            Status::SUPERSEDED => "superseded",
            Status::SUPERSEDES => "supersedes",
            Status::OBSOLETED => "obsoleted",
            Status::NONE => "unknown",
        }
    }

    fn from_str(val: String) -> Status {
        match val.as_str() {
            "wip" => Status::WIP,
            "decided" => Status::DECIDED,
            "completed" => Status::COMPLETED,
            "completes" => Status::COMPLETES,
            "superseded" => Status::SUPERSEDED,
            "supersedes" => Status::SUPERSEDES,
            "obsoleted" => Status::OBSOLETED,
            _ => Status::NONE,
        }
    }
}

pub struct Adr {
    pub path: String,
    pub content: String,
    pub title: String,
    pub status: Status,
    pub tags: String,
}

pub fn list_all_adr(dir: &str) -> io::Result<Vec<Adr>> {
    let mut results = std::vec::Vec::new();

    if Path::new(dir).is_dir() {
        let walker = WalkDir::new(dir).into_iter();
        for entry in walker.filter_entry(|e| !is_hidden(e)) {
            let entry = entry?;
            let metadata = entry.metadata().unwrap();
            if metadata.is_file() {
                let content: String = fs::read_to_string(entry.path()).unwrap();
                let adr = build_adr(String::from(entry.path().to_str().unwrap()), content)?;
                results.push(adr);
            }
        }
    }

    Ok(results)
}

fn build_adr(path: String, content: String) -> io::Result<Adr> {
    //get the title
    lazy_static! {
        static ref RE: Regex = Regex::new(r"= (.+)").unwrap();
    }
    let val = String::from(&content);
    let cap = match RE.captures(&val) {
        Some(val) => val[1].to_string(),
        None => {
            error!(get_logger(), "Unable to get title from [{}]", path);
            "None".to_string()
        }
    };

    //build the tags
    let tags = get_tags(&val);

    //build the status
    lazy_static! {
        static ref RE_STATUS: Regex = Regex::new(r"\{(.+)\}").unwrap();
    }
    let status = match RE_STATUS.captures(&val) {
        Some(val) => val[1].trim().to_string(),
        None => {
            debug!(get_logger(), "Unable to get status from [{}]", path);
            "None".to_string()
        }
    };

    //build the returned object
    let adr: Adr = Adr {
        path: path,
        content: content,
        title: cap,
        tags: tags,
        status: Status::from_str(status),
    };

    Ok(adr)
}

fn get_tags(val: &String) -> String {
    lazy_static! {
        static ref RE_TAGS: Regex = Regex::new(r"(\[tags]\#([^#]+)\#)").unwrap();
    }

    let mut tags = String::from("");
    for cap in RE_TAGS.captures_iter(val) {
        write!(tags, "{}, ", &cap[2]).unwrap();
    }

    tags
}

pub fn update_to_decided(adr_name: &str) -> io::Result<bool> {
    let mut f = File::open(adr_name)?;

    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();

    let contains = contents.contains("{wip}");
    if contains {
        let new_content = contents.replace("{wip}", "{decided}");
        fs::write(adr_name, new_content)?;
        info!(
            get_logger(),
            "Decision Record [{}] has been decided - Congrats!!", adr_name
        );
    } else {
        error!(
            get_logger(),
            "Decision Record [{}] has certainly not the right status and cannot be updated",
            adr_name
        );
    }

    Ok(contains)
}

pub fn superseded_by(adr_name: &str, by: &str) -> io::Result<()> {
    //check the decisino is decided
    let mut f = File::open(adr_name)?;
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();
    match contents.contains("{decided}") {
        true => {
            //manage the from
            let superseded_by = format!("{{superseded}} {}", by);
            update_adr_file(adr_name, &superseded_by)?;

            //manage the by
            let supersed = format!("{{supersedes}} {}", adr_name);
            update_adr_file(by, &supersed)?;

            info!(
                get_logger(),
                "Decision Record [{}] has been superseded by [{}]", adr_name, by
            );
        }
        false => {
            error!(
                get_logger(),
                "Decision Record [{}] has certainly not the right status and cannot be updated",
                adr_name
            );
        }
    }

    Ok(())
}

fn update_adr_file(adr_name: &str, tag_to_replace: &str) -> io::Result<()> {
    let mut contents = String::new();
    {
        let mut f = File::open(adr_name)?;
        f.read_to_string(&mut contents).unwrap();
    }
    let new_content = contents.replace("{decided}", tag_to_replace);
    fs::write(adr_name, new_content)?;

    Ok(())
}

pub fn completed_by(adr_name: &str, by: &str) -> io::Result<()> {
    //check the decisino is decided
    let mut f = File::open(adr_name)?;
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();
    match contents.contains("{decided}") {
        true => {
            //manage the from
            let completed_by = format!("{{completed}} {}", by);
            update_adr_file(adr_name, &completed_by)?;

            //manage the by
            let completes = format!("{{completes}} {}", adr_name);
            update_adr_file(by, &completes)?;

            info!(
                get_logger(),
                "Decision Record [{}] has been completed by [{}]", adr_name, by
            );
        }
        false => {
            error!(
                get_logger(),
                "Decision Record [{}] has certainly not the right status and cannot be updated",
                adr_name
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_get_seq() {
        let seq = super::extract_seq_id("01-my-decision.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq = super::extract_seq_id("00000010-my-decision.adoc").unwrap();
        assert_eq!(seq, 10);
        let seq = super::extract_seq_id("mypath/00000001-my-decision.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq = super::extract_seq_id("mypath/00000001-my-decision-594.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq = super::extract_seq_id("mypath/00000001-my-decision-594-full.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq = super::extract_seq_id("00000001-my-decision-594-full.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq =
            super::extract_seq_id("mypath/00000001/00000002-my-decision-594-full.adoc").unwrap();
        assert_eq!(seq, 1);

        let result =
            std::panic::catch_unwind(|| super::extract_seq_id("path/my-decision-full.adoc"));
        assert!(result.is_err());
    }

    #[test]
    fn test_format_decision_name() {
        let name = super::format_decision_name("my-decision").unwrap();
        assert_eq!(name, "my-decision");
        let name = super::format_decision_name("my decision").unwrap();
        assert_eq!(name, "my-decision");
        let name = super::format_decision_name("my Decision").unwrap();
        assert_eq!(name, "my-decision");
    }

    #[test]
    fn test_build_adr() {
        let content = "
        == ADR-MVA-507 Decide about ...
        
        *Status:* {wip}  *Date:* 2019-10-28
        ....
        
        [tags]#deployment view# [tags]#network# [tags]#security#";

        let adr_sut = super::build_adr("a_path".to_string(), content.to_string()).unwrap();

        assert_eq!(adr_sut.title, "ADR-MVA-507 Decide about ...");
        assert_eq!(adr_sut.path, "a_path");
        assert_eq!(adr_sut.content, content.to_string());
        assert_eq!(adr_sut.tags, "deployment view, network, security, ");
        assert_eq!(adr_sut.status, super::Status::WIP);
    }

    #[test]
    fn test_build_adr_wo_tags() {
        let content = "
        == ADR-MVA-507 Decide about ...
        
        *Status:* {wip}  *Date:* 2019-10-28
        ....";

        let adr_sut = super::build_adr("a_path".to_string(), content.to_string()).unwrap();

        assert_eq!(adr_sut.title, "ADR-MVA-507 Decide about ...");
        assert_eq!(adr_sut.path, "a_path");
        assert_eq!(adr_sut.content, content.to_string());
        assert_eq!(adr_sut.tags, "");
    }
}
