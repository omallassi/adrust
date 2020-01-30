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
pub fn create_adr(cfg: AdrToolConfig, name: &str, path_to_template: &Path, src_dir: &Path) -> io::Result<bool> {
    //specify last seq_id , the rest of the config (use_prefix and width can be get from the method)
    let name = match format_decision_name(cfg, name) {
        Ok(name) => name,
        Err(_why) => panic!(format!("Problem while formatting name [{}]", name)),
    };
    let target_path = src_dir.join(format!("{}.adoc", name));
    let is_target_file = target_path.is_file();
    if !is_target_file {
        match path_to_template.exists() {
            true => {
                fs::copy(path_to_template, &target_path)?;
                //need to update the title of the ADR with specified name. there is certainly a better way
                //reading again the file... 
                let adr_content = fs::read_to_string(&target_path).unwrap();
                let content = adr_content.replacen("== {%%ADR TITLE%%}", &name, 1);
                fs::write(&target_path, content).unwrap();
                //
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
        static ref RE: Regex = Regex::new(r"(\d+)-{1}").unwrap();
    }

    let mut id: usize = 0;
    if let Some(cap) = RE.captures(name) {
        debug!(get_logger(), "found first match [{}]", cap[1].to_string());
        id = cap[1].to_string().parse().unwrap();
    }
    else
    {
        debug!(get_logger(), "Unable to extract_seq_id from [{}]", name);
    }

    Ok(id)
}

fn extract_seq_id_from_all(adr_paths: Vec<String>) -> usize {
    let mut seq = 0;
    for path in adr_paths.iter(){
        //extract the seq_id
        let extracted_seq_id = extract_seq_id(path.as_str()).unwrap();
        if extracted_seq_id > seq {
            debug!(get_logger(), "got seq_id {} - compared to {}", extracted_seq_id, seq);
            seq = extracted_seq_id;
        }
    }

    debug!(get_logger(), "returned seq_id [{}]", seq);

    seq
}

fn get_last_seq_id(dir: &Path) -> usize {
    let adrs: Vec<Adr> = list_all_adr_from_path(dir).unwrap();
    let adrs_paths = adrs.iter().map(|adr| {
                                    let path = adr.path.clone();
                                    path
                                }).collect::<Vec<String>>();

    extract_seq_id_from_all(adrs_paths)
}

pub fn format_decision_name(cfg: AdrToolConfig, name: &str) -> Result<String> {
    let mut prefix = String::new();
    if cfg.use_id_prefix {
    let last_seq_id = get_last_seq_id(Path::new(cfg.adr_src_dir.as_str()));
        prefix = format!("{:0>width$}-", last_seq_id + 1, width = cfg.id_prefix_width);  //"{:0width$}", x, width = width
        debug!(get_logger(), "got seq number [{}]", prefix);
    }

    let name = name.to_ascii_lowercase();
    let name = name.replace(" ", "-");
    let name = format!("{}{}", prefix, name);

    Ok( name.to_string() )
}

fn is_ok(entry: &DirEntry) -> bool {

    let metadata = entry.metadata().unwrap();
    let is_dir = metadata.is_dir();
    
    let is_hidden = entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false);

    let is_adoc = entry
        .file_name()
        .to_str()
        .map(|s| s.ends_with(".adoc"))
        .unwrap_or(false);
    
    (is_dir && !is_hidden) || (is_adoc && !is_hidden)
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
    let dir_path = Path::new(dir);

    list_all_adr_from_path(dir_path)    
}

fn list_all_adr_from_path(dir: &Path) -> io::Result<Vec<Adr>> {
    let mut results = std::vec::Vec::new();
    
    if dir.is_dir() {
        let walker = WalkDir::new(dir).follow_links(true).into_iter();
        for entry in walker.filter_entry(|e| is_ok(e)) {
            let entry = entry?;
            debug!(get_logger(), "got file [{:?}]", entry.path());
            let metadata = entry.metadata().unwrap();
            if metadata.is_file() {
                match fs::read_to_string(entry.path()){
                    Ok(content) => {
                        let adr = build_adr(String::from(entry.path().to_str().unwrap()), content)?;
                        results.push(adr);
                    },
                    Err(_why) => {
                        debug!(get_logger(), "Unable to read file [{:?}]", entry.path());
                    }
                };

                
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
        assert_eq!(seq, 2
        );

        let seq =
        super::extract_seq_id("path/my-decision-full.adoc").unwrap();
        assert_eq!(seq, 0);

        // let result =
        //     std::panic::catch_unwind(|| super::extract_seq_id("path/my-decision-full.adoc"));
        // assert!(result.is_err());
    }

    #[test]
    fn test_extract_seq_id_from_all_1() {
        let mut paths = Vec::new();
        paths.push(String::from("01-my-decision.adoc"));
        paths.push(String::from("00000010-my-decision.adoc"));
        paths.push(String::from("mypath/00000002-my-decision.adoc"));
        paths.push(String::from("mypath/00000003-my-decision-594.adoc"));
        paths.push(String::from("mypath/00000001-my-decision-594-full.adoc"));
        paths.push(String::from("00000001-my-decision-594-full.adoc"));
        paths.push(String::from("mypath/00000001/00000002-my-decision-594-full.adoc"));
        paths.push(String::from("path/my-decision-full.adoc"));

        let seq = super::extract_seq_id_from_all(paths);
        assert_eq!(seq, 10);
    }

    #[test]
    fn test_extract_seq_id_from_all_2() {
        let mut paths = Vec::new();
        paths.push(String::from("attemtps.adoc"));
        paths.push(String::from("attemtps43.adoc"));
        paths.push(String::from("this-is-a-sample-12.adoc"));
        paths.push(String::from("this-is-a-sample-14.adoc"));
        paths.push(String::from("this-is-a-sample-17.adoc"));
        paths.push(String::from("this-is-a-smple4.adoc"));
        paths.push(String::from(""));
        paths.push(String::from("this-is-a-smple7.adoc"));

        let seq = super::extract_seq_id_from_all(paths);
        assert_eq!(seq, 0);
    }

    #[test]
    fn test_format_decision_name() {
        let mut cfg: super::AdrToolConfig = adr_config::config::get_config();
        cfg.use_id_prefix = false;
        let name = super::format_decision_name(cfg, "my-decision").unwrap();
        assert_eq!(name, "my-decision");

        let mut cfg: super::AdrToolConfig = adr_config::config::get_config();
        cfg.use_id_prefix = false;
        let name = super::format_decision_name(cfg, "my decision").unwrap();
        assert_eq!(name, "my-decision");

        let mut cfg: super::AdrToolConfig = adr_config::config::get_config();
        cfg.use_id_prefix = false;
        let name = super::format_decision_name(cfg, "my Decision").unwrap();
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
