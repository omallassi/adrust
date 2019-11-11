extern crate slog;
extern crate slog_term;
use slog::*;

use std::fs::{self, File};
use std::io::{self};
use std::io::prelude::*;
use std::path::Path;

extern crate regex;
use regex::Regex;

lazy_static! {
    static ref LOGGER : slog::Logger = { 
        let decorator = slog_term::PlainSyncDecorator::new(std::io::stdout());
        let drain = slog_term::FullFormat::new(decorator).build().fuse();

        let log = slog::Logger::root(drain, o!());

        log
    };
}

///
/// Creates the file (based on template file). Returns true if file is created, false if not 
/// (because target file already exists...)
pub fn create_adr(name: &str, templates_dir: &Path, src_dir: &Path) -> io::Result<(bool)> {
    let name = match format_decision_name(name) {
        Ok(name) => name,
        Err(_why) => panic!(format!("Problem while formatting name [{}]", name)),
    };
    let target_path = src_dir.join(format!("{}.adoc", name));
    let is_target_file = target_path.is_file();
    if !is_target_file {
        fs::copy(templates_dir.join("adr-template-v0.1.adoc"), &target_path)?;
        info!(LOGGER, "New ADR {:?} created", target_path);
    }
    else {
        error!(LOGGER, "Decision already exists. Please use another name", );
    }

    Ok(!is_target_file)
}

fn extract_seq_id(name: &str) -> Result<(usize)> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(\d+)").unwrap();
    }

    let cap = match RE.captures(name) {
        Some(val) => val, 
        None => {
            error!(LOGGER, "Unable to extract_seq_id from [{}]", name);
            panic!();
        },
    };

    debug!(LOGGER, "found first match [{}]", cap[0].to_string());
    let id: usize = cap[0].to_string().parse().unwrap();

    Ok(id)
}

pub fn format_decision_name(name: &str) -> Result<(String)> {
    let name = name.to_ascii_lowercase();
    let name = name.replace(" ", "-");

    Ok(name.to_string())
}

pub fn list_all_adr(dir: &str) -> io::Result<(Vec<String>)> {
    let mut results = std::vec::Vec::new();

    if Path::new(dir).is_dir() {
        for entry in fs::read_dir(Path::new(dir))? {
            let entry_path = entry?.path();
            let path = entry_path.display(); //display() is not the best 

            results.push(format!("{}", &path));
        }
    }

    Ok(results)
}

pub fn update_to_decided(adr_name: &str) -> io::Result<(bool)> {
    let mut f = File::open(adr_name)?;

    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();
    let contains = contents.contains("{cl-wip}");
    if contains {
        let new_content = contents.replace("{cl-wip}", "{cl-decided}");
        fs::write(adr_name, new_content)?;
        info!(LOGGER, "Decision Record [{}] has been decided - Congrats!!", adr_name);
    }
    else {
        error!(LOGGER, "Decision Record [{}] has certainly not the right status and cannot be updated", adr_name);
    }

    Ok(contains)
}

pub fn superseded_by(adr_name: &str, by: &str) -> io::Result<()> {
    //manage the adr_name
    let mut contents = String::new();
    {
        let mut f = File::open(adr_name)?;
        f.read_to_string(&mut contents).unwrap();
    }
    let superseded_by = format!("{{cl-superseded}} {}", by);
    let new_content = contents.replace("{cl-decided}", &superseded_by);
    fs::write(adr_name, new_content)?;

    //manage the by
    let mut contents = String::new();
    {
        let mut f = File::open(by)?;
        f.read_to_string(&mut contents).unwrap();
    }
    let supersed = format!("{{cl-supersedes}} {}", adr_name);
    let new_content = contents.replace("{cl-decided}", &supersed);
    fs::write(by, new_content)?;

    info!(LOGGER, "Decision Record [{}] has been superseded by [{}]", adr_name, by);

    Ok(())
}

pub fn completed_by(_adr_name: &str, _by: &str) -> io::Result<()> {
    println!("et hops depuis un autre crate");

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
        let seq = super::extract_seq_id("mypath/00000001/00000002-my-decision-594-full.adoc").unwrap();
        assert_eq!(seq, 1);

        let result = std::panic::catch_unwind(|| super::extract_seq_id("path/my-decision-full.adoc"));
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
}