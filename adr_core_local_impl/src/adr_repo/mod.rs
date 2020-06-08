extern crate slog;
extern crate slog_term;
use slog::*;

use std::collections::HashMap;
use std::fs::{self};
use std::io::{self};
use std::path::Path;

extern crate regex;
use regex::Regex;

use walkdir::{DirEntry, WalkDir};

extern crate adr_config;
use adr_config::config::AdrToolConfig;

use chrono::prelude::*;

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

/// Creates the file (based on template file). Returns true if file is created, false if not (e.g. target file already exists...)
///
/// # Arguments
///
/// * `cfg` - The whole config object
/// * `title`- the title of the ADR (specified by the user)
/// *
///
pub fn create_adr(cfg: AdrToolConfig, title: &str) -> io::Result<bool> {
    let adr_template_dir = &cfg.adr_template_dir.as_str();
    let adr_template_file = &cfg.adr_template_file.as_str();

    let path_to_template = Path::new(adr_template_dir);
    let path_to_template = path_to_template.join(adr_template_file);
    let path_to_template = path_to_template.as_path();

    let src_dir = Path::new(&cfg.adr_src_dir);

    //specify lcargo buildast seq_id , the rest of the config (use_prefix and width can be get from the method)
    let name = match format_decision_name(cfg.clone(), title) {
        Ok(name) => name,
        Err(_why) => panic!(format!("Problem while formatting name [{}]", title)),
    };
    let target_path = src_dir.join(format!("{}.adoc", name));
    let is_target_file = target_path.is_file();
    if !is_target_file {
        if path_to_template.exists() {
            match fs::copy(path_to_template, &target_path) {
                Ok(_val) => debug!(
                    get_logger(),
                    "Copy template file from [{:?}] to [{:?}]", &path_to_template, &target_path
                ),
                Err(_why) => error!(
                    get_logger(),
                    "Unable to copy template from [{:?}] to [{:?}]",
                    &path_to_template,
                    &target_path
                ),
            };
            //build the Adr (and force the parsing)
            let mut new_adr = match build_adr(Path::new(&cfg.adr_src_dir), &target_path) {
                Ok(adr) => adr,
                Err(why) => {
                    error!(
                        get_logger(),
                        "Got error [{:?}] while getting ADR [{:?}]", why, target_path
                    );
                    panic!();
                }
            };

            new_adr.update_title(title);

            debug!(get_logger(), "Want to create ADR {:?}", &target_path);
            match fs::write(&target_path, new_adr.content) {
                Ok(_val) => info!(get_logger(), "New ADR [{:?}] created", target_path),
                Err(why) => {
                    error!(
                        get_logger(),
                        "Unable to create ADR [{:?}] - error [{:?}]", target_path, why
                    );
                }
            };
        //
        } else {
            error!(
                get_logger(),
                "[{}] was not found",
                path_to_template.to_string_lossy()
            );
        }
    } else {
        error!(
            get_logger(),
            "Decision already exists. Please use another name",
        );
    }

    Ok(!is_target_file)
}

fn get_seq_id_from_name(name: &str) -> Result<usize> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^(\d+)-{1}").unwrap();
    }

    let mut id: usize = 0;
    if let Some(cap) = RE.captures(name) {
        debug!(get_logger(), "found first match [{}]", cap[1].to_string());
        id = cap[1].to_string().parse().unwrap();
    } else {
        debug!(get_logger(), "Unable to extract_seq_id from [{}]", name);
    }

    Ok(id)
}

fn get_last_seq_id(adrs: Vec<Adr>) -> usize {
    get_seq_id_from_name(&adrs[adrs.len() - 1].file_name).unwrap()
}

fn sort_by_id(mut adrs: Vec<Adr>) -> Vec<Adr> {
    adrs.sort_by(|a, b| a.file_id.cmp(&b.file_id));
    adrs
}

fn format_decision_name(cfg: AdrToolConfig, name: &str) -> Result<String> {
    let mut prefix = String::new();
    if cfg.use_id_prefix {
        let adr_vec = list_all_adr(Path::new(cfg.adr_src_dir.as_str())).unwrap();
        let last_seq_id = get_last_seq_id(adr_vec);
        prefix = format!("{:0>width$}-", last_seq_id + 1, width = cfg.id_prefix_width); //"{:0width$}", x, width = width
        debug!(get_logger(), "got seq number [{}]", prefix);
    }

    let name = name.to_ascii_lowercase();
    let name = name.replace(" ", "-");
    let name = format!("{}{}", prefix, name);

    Ok(name.to_string())
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

pub fn get_tags_popularity(base_path: &Path) -> Result<HashMap<String, u32>> {
    let mut popularity: HashMap<String, u32> = HashMap::new();
    for adr in list_all_adr(base_path)? {
        for tag in adr.tags_array.iter() {
            popularity
                .entry(tag.to_string())
                .and_modify(|e| *e += 1)
                .or_insert(1);
        }
    }

    Ok(popularity)
}

pub fn list_all_adr(dir: &Path) -> io::Result<Vec<Adr>> {
    let mut results = std::vec::Vec::new();

    if dir.is_dir() {
        let walker = WalkDir::new(dir).follow_links(true).into_iter();
        for entry in walker.filter_entry(|e| is_ok(e)) {
            let entry = entry?;
            debug!(get_logger(), "got file [{:?}]", entry.path());
            let metadata = entry.metadata().unwrap();
            if metadata.is_file() {
                match build_adr(dir, entry.path()) {
                    Ok(adr) => {
                        results.push(adr);
                    }
                    Err(_why) => {
                        debug!(get_logger(), "Unable to read file [{:?}]", entry.path());
                    }
                };
            }
        }
    }

    results = sort_by_id(results);

    Ok(results)
}

/// Given a complete `full_path` to a file, returns the difference compared to `base_path`.
///
/// # Arguments
///
/// * `base_path` - The root directory where are all the ADRs. This is typically AdrToolConfig.adr_root_dir
/// * `file_path` - The full path of the file
///
/// # Example
///
/// ```
/// use adr_core::adr_repo::split_path;
///
/// let adr = split_path(std::path::Path::new("/tmp/adrs/"), std::path::Path::new("/tmp/adrs/my-sub-dir/my-decision.adoc"));
/// assert_eq!(adr.0, std::path::Path::new("/tmp/adrs/"));
/// assert_eq!(adr.1, std::path::Path::new("my-sub-dir/my-decision.adoc"));
/// ```
///
pub fn split_path<'a>(base_path: &'a Path, full_path: &'a Path) -> (&'a Path, &'a Path) {
    debug!(
        get_logger(),
        "Want to split_path[{:?}] and [{:?}] ", base_path, full_path
    );
    match full_path.starts_with(base_path) {
        true => (
            base_path,
            full_path.strip_prefix(base_path).unwrap_or(full_path),
        ),
        false => (base_path, full_path),
    }
}

/// Build an ADR object given the provided arguments. Inside the ADR struct `full_path` will be splitted into `file_path` and `base_path`
///
/// # Arguments
///
/// * `base_path` - The root directory where are all the ADRs. This is typically AdrToolConfig.adr_root_dir
/// * `file_path` - The full path of the file
///
/// # Example
///
/// ```
/// use adr_core::adr_repo::build_adr;
/// let adr = build_adr(std::path::Path::new("/tmp/adrs/"), std::path::Path::new("/tmp/adrs/my-sub-dir/my-decision.adoc"));
/// ```
pub fn build_adr(base_path: &Path, full_path: &Path) -> io::Result<Adr> {
    debug!(
        get_logger(),
        "Want to create ADR from [{}] ",
        full_path.display()
    );
    let content = fs::read_to_string(full_path)?;

    //build the adr
    let splitted_file_path = split_path(base_path, full_path);
    let adr = Adr::from(
        String::from(splitted_file_path.0.to_str().unwrap()),
        String::from(splitted_file_path.1.to_str().unwrap()),
        content,
    );

    Ok(adr)
}

pub fn transition_to_decided(base_path: &Path, file_path: &str) -> io::Result<bool> {
    transition_to(TransitionStatus::DECIDED, base_path, file_path, "")
}

pub fn transition_to_superseded_by(
    base_path: &Path,
    file_path: &str,
    by: &str,
) -> io::Result<bool> {
    transition_to(TransitionStatus::SUPERSEDED, base_path, file_path, by)
}

pub fn transition_to_completed_by(base_path: &Path, file_path: &str, by: &str) -> io::Result<bool> {
    transition_to(TransitionStatus::COMPLETED, base_path, file_path, by)
}

pub fn transition_to_obsoleted(base_path: &Path, file_path: &str) -> io::Result<bool> {
    transition_to(TransitionStatus::CANCELLED, base_path, file_path, "")
}

pub fn transition_to(
    transition: TransitionStatus,
    base_path: &Path,
    from_path: &str,
    by_path: &str,
) -> io::Result<bool> {
    let mut from_adr = match build_adr(base_path, Path::new(from_path)) {
        Ok(adr) => adr,
        Err(why) => {
            error!(
                get_logger(),
                "Got error [{:?}] while getting ADR [{}]", why, from_path
            );
            panic!();
        }
    };
    let from_old_status = from_adr.status.as_str();

    //if transition has been declined, we can stop here
    match from_adr.update_status(transition) {
        true => {
            debug!(
                get_logger(),
                "ADR [{}] has a new status [{}]",
                from_adr.path().as_str(),
                from_adr.status.as_str()
            );
            let transition_adr = |adr: &Adr, path: &str, old_status: &str| -> io::Result<bool> {
                match fs::write(path, &adr.content) {
                    Ok(_) => {
                        info!(
                            get_logger(),
                            "Transitioned [{}] from [{}] to [{}]",
                            adr.path().as_str(),
                            old_status,
                            adr.status.as_str()
                        );
                        Ok(true)
                    }
                    Err(_) => Ok(false),
                }
            };
            match by_path.is_empty() {
                true => transition_adr(&from_adr, from_path, from_old_status),
                false => {
                    let mut by_adr = build_adr(base_path, Path::new(by_path))?;
                    let by_old_status = by_adr.status.as_str();
                    //if transition has been declined, we can stop here
                    match by_adr.update_status(TransitionStatus::revert(transition)) {
                        true => {
                            from_adr.add_reference(format!("{}", by_adr.file_name).as_str());
                            by_adr.add_reference(format!("{}", from_adr.file_name).as_str());
                            Ok(transition_adr(&from_adr, from_path, from_old_status)?
                                == transition_adr(&by_adr, by_path, by_old_status)?)
                        }
                        false => {
                            error!(get_logger(), "ADR [{}] cannot be transitioned to [{:?}] - Status of [{:?}] is not [{:?}]", from_path, transition, by_path, TransitionStatus::DECIDED);
                            Ok(false)
                        }
                    }
                }
            }
        }
        false => {
            error!(
                get_logger(),
                "ADR [{}] cannot be transitioned to [{:?}]", from_path, transition
            );
            Ok(false)
        }
    }
}

#[derive(Debug, Default)]
pub struct Adr {
    //pub path: String, //the path from config.adr_root_dir (which is user dependant)
    pub file_id: usize,
    pub file_name: String,
    pub file_path: String,
    pub base_path: String,
    pub content: String,
    pub title: String,
    pub date: String,
    pub status: Status,
    pub state: AdrState,
    pub tags: String,
    pub tags_array: Vec<String>,
}

impl Adr {
    fn new() -> Adr {
        Adr {
            file_id: 0,
            file_name: String::new(),
            file_path: String::new(),
            base_path: String::new(),
            content: String::new(),
            title: String::new(),
            date: String::new(),
            status: Status::default(),
            state: AdrState::default(),
            tags: String::new(),
            tags_array: Vec::new(),
        }
    }

    pub fn from(base_path: String, file_path: String, content: String) -> Adr {
        let mut adr = Adr::new();

        lazy_static! {
            static ref RE_TITLE: Regex = Regex::new(r"= (.+)").unwrap();
            static ref RE_STATUS: Regex = Regex::new(r"\{(.+)\}").unwrap();
            static ref RE_DATE: Regex = Regex::new(r"([0-9]{4}-[0-9]{2}-[0-9]{2})").unwrap();
        }

        //set file/path properties
        adr.base_path = base_path;
        adr.file_path = file_path;
        adr.file_name = match Path::new(&adr.file_path)
            .file_name()
            .unwrap()
            .to_os_string()
            .into_string()
        {
            Ok(name) => name,
            Err(err) => panic!("Unexpected Error: {:?}", err),
        };
        adr.file_id = get_seq_id_from_name(&adr.file_name).unwrap();

        //set title/content
        adr.content = content;
        adr.title = match RE_TITLE.captures(&adr.content) {
            Some(val) => val[1].to_string(),
            None => {
                error!(
                    get_logger(),
                    "Unable to get title from base_path [{}] and file_path [{}]",
                    adr.base_path,
                    adr.file_path
                );
                "None".to_string()
            }
        };

        //set date
        adr.date = match RE_DATE.captures(&adr.content) {
            Some(val) => val[1].trim().to_string(),
            None => {
                debug!(
                    get_logger(),
                    "Unable to get date from base_path [{}] and file_path [{}]",
                    adr.base_path,
                    adr.file_path
                );
                "None".to_string()
            }
        };

        //set tags/tags_array
        let tags = Adr::get_tags(&adr.content);
        adr.tags = tags.0;
        adr.tags_array = tags.1;

        //set status/state
        adr.status = Status::from_str(match RE_STATUS.captures(&adr.content) {
            Some(val) => val[1].trim().to_string(),
            None => {
                debug!(
                    get_logger(),
                    "Unable to get status from base_path [{}] and file_path [{}]",
                    adr.base_path,
                    adr.file_path
                );
                "None".to_string()
            }
        });
        adr.state = AdrState {
            status: adr.status.clone(),
        };
        adr
    }

    pub fn path(&self) -> String {
        let full_path = Path::new(self.base_path.as_str()).join(self.file_path.as_str());
        return format!("{}", full_path.display());
    }

    pub fn get_tags(val: &String) -> (String, Vec<String>) {
        lazy_static! {
            static ref RE_TAGS: Regex = Regex::new(r"(\[tags]\#([^#]+)\#)").unwrap();
        }

        let mut tags_str = String::from("");
        for cap in RE_TAGS.captures_iter(val) {
            use std::fmt::Write;
            write!(tags_str, "#{} ", &cap[2]).unwrap();
        }

        let tags = tags_str
            .split('#')
            .filter(|s| s.len() > 0)
            .map(|s| s.to_string())
            .collect();

        (tags_str, tags)
    }

    pub fn update_status(&mut self, transition: TransitionStatus) -> bool {
        let current_status = format!("{{{status}}}", status = self.status.as_str()); //you escape { with a { and final status is {wip}  o_O
        let mut state = self.state;
        let has_been_modified = state.transition(transition);

        debug!(get_logger(), "Want transition [{:?}] - Adr State transitioned from [{:?}] to [{:?}] - has been modified [{:?}]", transition, self.state, state, has_been_modified);

        if has_been_modified {
            let new_status = format!("{{{status}}}", status = state.status.as_str());
            debug!(get_logger(), "Transitioned to [{}]", state.status.as_str());

            self.content = self
                .content
                .replace(current_status.as_str(), new_status.as_str());
            self.status = state.status;
            self.state = state;
            self.update_date(Utc::today());
            has_been_modified
        } else {
            debug!(get_logger(), "Transition has been declined");
            false
        }
    }

    pub fn add_reference(&mut self, adr_title: &str) {
        let current_status = format!("{{{status}}}", status = self.status.as_str()); //you escape { with a { and final status is {wip}  o_O
        let new_status = format!(
            "{updated_by} {by}",
            updated_by = current_status.as_str(),
            by = adr_title
        );
        debug!(
            get_logger(),
            "Want to add reference - current status [{:?}] - new status [{:?}]",
            current_status,
            new_status
        );

        self.content = self
            .content
            .replace(current_status.as_str(), new_status.as_str());
    }

    pub fn update_date(&mut self, today: Date<Utc>) {
        let new_date = today.format("%Y-%m-%d").to_string();
        debug!(get_logger(), "Want to update ADR to date [{}]", new_date);

        self.date = new_date;
        let re = Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap();
        self.content = re
            .replace(self.content.as_str(), self.date.as_str())
            .as_ref()
            .to_owned();
    }

    pub fn update_title(&mut self, title: &str) {
        let new_title = "".to_owned() + title;

        self.content = self
            .content
            .replacen(self.title.as_str(), new_title.as_str(), 1);
        self.title = new_title;
    }
}

impl Clone for Adr {
    fn clone(&self) -> Adr {
        Adr {
            file_id: self.file_id.clone(),
            file_name: String::from(self.file_name.as_str()),
            file_path: String::from(self.file_path.as_str()),
            base_path: String::from(self.base_path.as_str()),
            content: String::from(self.content.as_str()),
            title: String::from(self.title.as_str()),
            date: String::from(self.date.as_str()),
            tags: String::from(self.tags.as_str()),
            tags_array: self.tags_array.clone(),
            status: self.state.status.clone(),
            state: self.state.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TransitionStatus {
    DECIDED,
    COMPLETED,
    COMPLETES,
    SUPERSEDED,
    SUPERSEDES,
    CANCELLED,
    NONE,
}

impl TransitionStatus {
    fn revert(transition: TransitionStatus) -> TransitionStatus {
        match transition {
            TransitionStatus::COMPLETED => TransitionStatus::COMPLETES,
            TransitionStatus::COMPLETES => TransitionStatus::COMPLETED,

            TransitionStatus::SUPERSEDED => TransitionStatus::SUPERSEDES,
            TransitionStatus::SUPERSEDES => TransitionStatus::SUPERSEDED,

            _ => transition,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match *self {
            TransitionStatus::DECIDED => "decided",
            TransitionStatus::COMPLETED => "completed",
            TransitionStatus::COMPLETES => "completes",
            TransitionStatus::SUPERSEDED => "superseded",
            TransitionStatus::SUPERSEDES => "supersedes",
            TransitionStatus::CANCELLED => "cancelled",
            _ => "none",
        }
    }

    pub fn from_str(val: String) -> TransitionStatus {
        match val.as_str() {
            "decided" => TransitionStatus::DECIDED,
            "completed" => TransitionStatus::COMPLETED,
            "completes" => TransitionStatus::COMPLETES,
            "superseded" => TransitionStatus::SUPERSEDED,
            "supersedes" => TransitionStatus::SUPERSEDES,
            "cancelled" => TransitionStatus::CANCELLED,
            _ => TransitionStatus::NONE,
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Status {
    WIP,
    DECIDED,
    COMPLETED,
    COMPLETES,
    SUPERSEDED,
    SUPERSEDES,
    CANCELLED,
    NONE,
}

impl Default for Status {
    fn default() -> Self {
        Status::WIP
    }
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
            Status::CANCELLED => "obsoleted",
            Status::NONE => "unknown",
        }
    }

    pub fn from_str(val: String) -> Status {
        match val.as_str() {
            "wip" => Status::WIP,
            "decided" => Status::DECIDED,
            "completed" => Status::COMPLETED,
            "completes" => Status::COMPLETES,
            "superseded" => Status::SUPERSEDED,
            "supersedes" => Status::SUPERSEDES,
            "obsoleted" => Status::CANCELLED,
            _ => Status::NONE,
        }
    }
}

pub trait State {
    fn transition(&mut self, transition: TransitionStatus) -> bool;

    fn build(status: Status) -> AdrState;
}

#[derive(Debug, Copy, Clone)]
pub struct AdrState {
    status: Status,
}

impl Default for AdrState {
    fn default() -> Self {
        AdrState {
            status: Status::WIP,
        }
    }
}

impl State for AdrState {
    fn transition(&mut self, transition: TransitionStatus) -> bool {
        let current_state = self.status.as_str();
        let current_status = &self.status;

        let mut has_been_modified = true;

        let next_status = match current_status {
            Status::WIP => match transition {
                TransitionStatus::DECIDED => {
                    self.status = Status::DECIDED;
                    Status::DECIDED
                }
                TransitionStatus::CANCELLED => {
                    self.status = Status::CANCELLED;
                    Status::CANCELLED
                }
                _ => {
                    has_been_modified = false;
                    Status::WIP
                }
            },
            Status::DECIDED => match transition {
                TransitionStatus::COMPLETED => {
                    self.status = Status::COMPLETED;
                    Status::COMPLETED
                }
                TransitionStatus::COMPLETES => {
                    self.status = Status::COMPLETES;
                    Status::COMPLETES
                }
                TransitionStatus::CANCELLED => {
                    self.status = Status::CANCELLED;
                    Status::CANCELLED
                }
                TransitionStatus::SUPERSEDED => {
                    self.status = Status::SUPERSEDED;
                    Status::SUPERSEDED
                }
                TransitionStatus::SUPERSEDES => {
                    self.status = Status::SUPERSEDES;
                    Status::SUPERSEDES
                }
                _ => {
                    has_been_modified = false;
                    Status::DECIDED
                }
            },
            Status::COMPLETED => match transition {
                TransitionStatus::SUPERSEDED => {
                    self.status = Status::SUPERSEDED;
                    Status::SUPERSEDED
                }
                TransitionStatus::CANCELLED => {
                    self.status = Status::CANCELLED;
                    Status::CANCELLED
                }
                _ => {
                    has_been_modified = false;
                    Status::COMPLETED
                }
            },
            Status::COMPLETES => match transition {
                TransitionStatus::CANCELLED => {
                    self.status = Status::CANCELLED;
                    Status::CANCELLED
                }
                TransitionStatus::SUPERSEDED => {
                    self.status = Status::SUPERSEDED;
                    Status::SUPERSEDED
                }
                _ => {
                    has_been_modified = false;
                    Status::COMPLETES
                }
            },
            Status::SUPERSEDED => match transition {
                TransitionStatus::CANCELLED => {
                    self.status = Status::CANCELLED;
                    Status::CANCELLED
                }
                _ => {
                    has_been_modified = false;
                    Status::SUPERSEDED
                }
            },
            Status::SUPERSEDES => match transition {
                TransitionStatus::CANCELLED => {
                    self.status = Status::CANCELLED;
                    Status::CANCELLED
                }
                _ => {
                    has_been_modified = false;
                    Status::SUPERSEDES
                }
            },
            Status::CANCELLED => match transition {
                _ => {
                    has_been_modified = false;
                    Status::CANCELLED
                }
            },

            _ => {
                has_been_modified = false;
                Status::NONE
            }
        };

        self.status = next_status;
        debug!(
            get_logger(),
            "transition [{:?}] has been called from [{:?}] to [{:?}]",
            transition,
            current_state,
            self.status
        );

        has_been_modified
    }

    fn build(status: Status) -> AdrState {
        AdrState { status: status }
    }
}

impl std::cmp::PartialEq for AdrState {
    fn eq(&self, other: &Self) -> bool {
        self.status == other.status
    }
}

#[cfg(test)]
mod tests {
    extern crate tempdir;
    use std::path::PathBuf;
    use tempdir::TempDir;

    use crate::adr_repo::*;
    const ADOC_TMPL_NOTAG: &str = ":docinfo1:
    :wip: pass:quotes[[.label.wip]#In Progress#]
    :decided: pass:q[[.label.decided]#Decided#]
    :completed: pass:q[[.label.updated]#Completed By#]
    :completes: pass:q[[.label.updated]#Completes#]
    :supersedes: pass:q[[.label.updated]#Supersedes#]
    :superseded: pass:q[[.label.obsoleted]#Superseded By#]
    :obsoleted: pass:q[[.label.obsoleted]#Obsolete#]

    = short title of solved problem and solution

    *Status:* {decided} *Date:* 2019-10-28
    ...";

    const ADOC_TMPL_TAG: &str = ":docinfo1:
    :wip: pass:quotes[[.label.wip]#In Progress#]
    :decided: pass:q[[.label.decided]#Decided#]
    :completed: pass:q[[.label.updated]#Completed By#]
    :completes: pass:q[[.label.updated]#Completes#]
    :supersedes: pass:q[[.label.updated]#Supersedes#]
    :superseded: pass:q[[.label.obsoleted]#Superseded By#]
    :obsoleted: pass:q[[.label.obsoleted]#Obsolete#]

    = short title of solved problem and solution

    *Status:* {wip} *Date:* 2019-10-28

    [tags]#tag1# [tags]#tag2# [tags]#tag3#
    ...";

    #[test]
    fn test_adr_update_status() {
        let mut adr_sut = Adr::new();
        adr_sut.file_name = String::from("/a");
        adr_sut.base_path = String::from("/tmp/n");
        adr_sut.file_path = String::from("/a");
        adr_sut.content = String::from("== ADR-MVA-507 Decide about ...\n\n*Status:* {wip} *Date:* 2019-10-28\n\n[cols=\",\",options=...");
        adr_sut.title = String::from("String::from(self.title.as_str())");
        adr_sut.date = String::from("2023-10-28");

        let update_true = adr_sut.update_status(TransitionStatus::DECIDED);

        assert_eq!(adr_sut.status, Status::DECIDED);
        assert_eq!(
            adr_sut.state,
            AdrState {
                status: Status::DECIDED
            }
        );
        assert_eq!(adr_sut.content.contains(Status::DECIDED.as_str()), true);
        assert_eq!(update_true, true);
    }

    #[test]
    fn test_adr_add_reference() {
        let mut adr_sut = Adr::new();
        adr_sut.file_name = String::from("/a");
        adr_sut.base_path = String::from("/tmp/n");
        adr_sut.file_path = String::from("/a");
        adr_sut.content = String::from("== ADR-MVA-507 Decide about ...\n\n*Status:* {decided} *Date:* 2019-10-28\n\n[cols=\",\",options=...");
        adr_sut.title = String::from("String::from(self.title.as_str())");
        adr_sut.date = String::from("2023-10-28");
        adr_sut.status = Status::DECIDED;
        adr_sut.state = AdrState {
            status: Status::DECIDED,
        };

        adr_sut.add_reference("by adr-num-123");

        assert_eq!(adr_sut.status, Status::DECIDED);
        assert_eq!(
            adr_sut.state,
            AdrState {
                status: Status::DECIDED
            }
        );

        let expected_status = "{decided} by adr-num-123 *Date:* 2019-10-28";
        assert_eq!(adr_sut.content.contains(expected_status), true);
    }

    #[test]
    fn test_state_machine() {
        let mut state = super::AdrState::build(super::Status::WIP);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::WIP
            }
        );
        state.transition(super::TransitionStatus::COMPLETED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::WIP
            }
        );
        state.transition(super::TransitionStatus::DECIDED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::DECIDED
            }
        );
        state.transition(super::TransitionStatus::SUPERSEDED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::SUPERSEDED
            }
        );
        state.transition(super::TransitionStatus::CANCELLED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::CANCELLED
            }
        );
    }

    #[test]
    fn test_state_machine_2() {
        let mut state = super::AdrState::build(super::Status::WIP);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::WIP
            }
        );
        state.transition(super::TransitionStatus::SUPERSEDED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::WIP
            }
        );
        state.transition(super::TransitionStatus::DECIDED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::DECIDED
            }
        );
        state.transition(super::TransitionStatus::COMPLETED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::COMPLETED
            }
        );
        state.transition(super::TransitionStatus::SUPERSEDED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::SUPERSEDED
            }
        );
        state.transition(super::TransitionStatus::CANCELLED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::CANCELLED
            }
        );
    }

    #[test]
    fn test_state_machine_wip_to_cancelled() {
        let mut state = super::AdrState::build(super::Status::WIP);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::WIP
            }
        );
        state.transition(super::TransitionStatus::CANCELLED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::CANCELLED
            }
        );
    }

    #[test]
    fn test_state_machine_decided_to_cancelled() {
        let mut state = super::AdrState::build(super::Status::DECIDED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::DECIDED
            }
        );
        state.transition(super::TransitionStatus::CANCELLED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::CANCELLED
            }
        );
    }

    #[test]
    fn test_state_machine_decided_to_completes() {
        let mut state = super::AdrState::build(super::Status::DECIDED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::DECIDED
            }
        );
        state.transition(super::TransitionStatus::COMPLETES);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::COMPLETES
            }
        );
    }

    #[test]
    fn test_state_machine_decided_to_supersedes() {
        let mut state = super::AdrState::build(super::Status::DECIDED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::DECIDED
            }
        );
        state.transition(super::TransitionStatus::SUPERSEDES);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::SUPERSEDES
            }
        );
    }

    #[test]
    fn test_state_machine_decided_to_fail() {
        let mut state = super::AdrState::build(super::Status::DECIDED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::DECIDED
            }
        );
        state.transition(super::TransitionStatus::NONE);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::DECIDED
            }
        );
    }

    #[test]
    fn test_state_machine_completed_to_cancelled() {
        let mut state = super::AdrState::build(super::Status::COMPLETED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::COMPLETED
            }
        );
        state.transition(super::TransitionStatus::CANCELLED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::CANCELLED
            }
        );
    }

    #[test]
    fn test_state_machine_superseded_to_fail() {
        let mut state = super::AdrState::build(super::Status::SUPERSEDED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::SUPERSEDED
            }
        );
        state.transition(super::TransitionStatus::DECIDED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::SUPERSEDED
            }
        );
    }

    #[test]
    fn test_state_machine_cancelled_to_cancelled() {
        let mut state = super::AdrState::build(super::Status::CANCELLED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::CANCELLED
            }
        );
        state.transition(super::TransitionStatus::DECIDED);
        assert_eq!(
            state,
            super::AdrState {
                status: super::Status::CANCELLED
            }
        );
    }

    #[test]
    fn test_get_seq() {
        let seq = super::get_seq_id_from_name("01-my-decision.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq = super::get_seq_id_from_name("00000010-my-decision.adoc").unwrap();
        assert_eq!(seq, 10);
        let seq = super::get_seq_id_from_name("00000001-my-decision.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq = super::get_seq_id_from_name("00000001-my-decision-594.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq = super::get_seq_id_from_name("00000001-my-decision-594-full.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq = super::get_seq_id_from_name("00000001-my-decision-594-full.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq = super::get_seq_id_from_name("00000002-my-decision-594-full.adoc").unwrap();
        assert_eq!(seq, 2);

        let seq = super::get_seq_id_from_name("my-decision-full.adoc").unwrap();
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
        paths.push(String::from(
            "mypath/00000001/00000002-my-decision-594-full.adoc",
        ));
        paths.push(String::from("path/my-decision-full.adoc"));
        paths.push(String::from("path/my-decision-543-0.adoc"));

        let mut adr_vec = Vec::new();
        for adr in paths.into_iter() {
            adr_vec.push(super::Adr::from(
                String::from("/adr/"),
                String::from(adr),
                String::from(ADOC_TMPL_NOTAG),
            ));
        }

        adr_vec = super::sort_by_id(adr_vec);
        let seq = super::get_last_seq_id(adr_vec);
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
        paths.push(String::from("this-is-a-smple7.adoc"));

        let mut adr_vec = Vec::new();
        for adr in paths.into_iter() {
            adr_vec.push(super::Adr::from(
                String::from("/adr/"),
                String::from(adr),
                String::from(ADOC_TMPL_NOTAG),
            ));
        }

        let seq = super::get_last_seq_id(adr_vec);
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
    fn test_build_adr_from_adr_constructor() {
        let content = "
        == ADR-MVA-507 Decide about ...

        *Status:* {wip}  *Date:* 2019-10-28
        ....
        bug there is another date 2119-10-28
        [tags]#deployment view# [tags]#network# [tags]#security#";

        let adr_sut = super::Adr::from(
            "base_path".to_string(),
            "a_path".to_string(),
            content.to_string(),
        );

        assert_eq!(adr_sut.title, "ADR-MVA-507 Decide about ...");
        assert_eq!(adr_sut.date, "2019-10-28");
        assert_eq!(adr_sut.base_path, "base_path");
        assert_eq!(adr_sut.file_path, "a_path");
        assert_eq!(adr_sut.content, content.to_string());
        assert_eq!(adr_sut.tags, "#deployment view #network #security ");
        assert_eq!(adr_sut.status, super::Status::WIP);
    }

    #[test]
    fn test_build_adr() {
        let src = match TempDir::new("my_src_folder") {
            Ok(src) => src,
            Err(why) => {
                println!("Unable to get src dir [{}]", why);
                panic!(why);
            }
        };

        let to = PathBuf::from(src.path()).join("decided.adoc");
        fs::write(to.as_path(), ADOC_TMPL_NOTAG).unwrap();

        println!("Want to work with [{}]", to.display());

        let adr = super::build_adr(src.path(), to.as_path()).unwrap();
        assert_eq!(Status::DECIDED, adr.status);
        assert_eq!("short title of solved problem and solution", adr.title);
        assert_eq!("2019-10-28", adr.date);
        assert_eq!(format!("{}", src.path().display()), adr.base_path);
        assert_eq!("decided.adoc", adr.file_path);
        assert_eq!("", adr.tags);
    }

    #[test]
    fn test_create_adr_wo_prefix() {
        let src = match TempDir::new("my_src_folder") {
            Ok(src) => {
                println!("Working with src dir [{}]", src.path().display());
                src
            }
            Err(why) => {
                println!("Unable to get src dir [{}]", why);
                panic!(why);
            }
        };
        //set config
        let config = AdrToolConfig {
            log_level: 6,
            //adr_root_dir: format!("{}", src.path().display()),
            adr_src_dir: format!("{}", src.path().display()),
            adr_template_dir: format!("{}", src.path().display()),
            adr_template_file: String::from("template.adoc"),
            adr_search_index: format!("{}", src.path().display()),
            use_id_prefix: false,
            id_prefix_width: 3,
        };

        let to = PathBuf::from(src.path()).join("template.adoc");
        fs::write(to.as_path(), ADOC_TMPL_NOTAG).unwrap();

        //test
        let created = super::create_adr(config, "title of the ADR");
        //
        assert!(created.unwrap());
        assert_eq!(true, src.path().exists());
        assert_eq!(true, src.path().join("title-of-the-adr.adoc").exists());
    }

    #[test]
    fn test_create_adr_w_prefix() {
        let src = match TempDir::new("my_src_folder") {
            Ok(src) => {
                println!("Working with src dir [{}]", src.path().display());
                src
            }
            Err(why) => {
                println!("Unable to get src dir [{}]", why);
                panic!(why);
            }
        };

        //set config
        let config = AdrToolConfig {
            log_level: 6,
            //adr_root_dir: format!("{}", src.path().display()),
            adr_src_dir: format!("{}", src.path().display()),
            adr_template_dir: format!("{}", src.path().display()),
            adr_template_file: String::from("template.adoc"),
            adr_search_index: format!("{}", src.path().display()),
            use_id_prefix: true,
            id_prefix_width: 3,
        };

        let to = PathBuf::from(src.path()).join("template.adoc");
        fs::write(to.as_path(), ADOC_TMPL_NOTAG).unwrap();

        //set a couple of already present files
        let to = PathBuf::from(src.path()).join("001-ADR-1.adoc");
        fs::write(to.as_path(), ADOC_TMPL_NOTAG).unwrap();
        let to = PathBuf::from(src.path()).join("003-ADR-2.adoc");
        fs::write(to.as_path(), ADOC_TMPL_NOTAG).unwrap();

        //test
        let created = super::create_adr(config, "title of the ADR");
        //
        assert!(created.unwrap());
        assert_eq!(true, src.path().exists());
        assert_eq!(true, src.path().join("004-title-of-the-adr.adoc").exists());
    }

    #[test]
    fn test_get_tags_popularity() {
        let src = match TempDir::new("my_src_folder") {
            Ok(src) => {
                println!("Working with src dir [{}]", src.path().display());
                src
            }
            Err(why) => {
                println!("Unable to get src dir [{}]", why);
                panic!(why);
            }
        };

        //set a couple of already present files
        let to = PathBuf::from(src.path()).join("001-ADR-1.adoc");
        fs::write(to.as_path(), ADOC_TMPL_TAG).unwrap();
        let to = PathBuf::from(src.path()).join("003-ADR-2.adoc");
        fs::write(to.as_path(), ADOC_TMPL_TAG).unwrap();
        let to = PathBuf::from(src.path()).join("004-ADR-2.adoc");
        fs::write(to.as_path(), ADOC_TMPL_TAG).unwrap();

        //test
        let tags = super::get_tags_popularity(src.path()).unwrap();
        //
        assert_eq!(3, tags.len());
        assert_eq!(Some(&3), tags.get("tag1 "));
        assert_eq!(Some(&3), tags.get("tag2 "));
        assert_eq!(Some(&3), tags.get("tag3 "));
    }

    #[test]
    fn test_build_adr_wo_tags() {
        let content = "
        == ADR-MVA-507 Decide about ...

        *Status:* {wip}  *Date:* 2019-10-28
        ....";

        let adr_sut = super::Adr::from(
            "base_path".to_string(),
            "a_path".to_string(),
            content.to_string(),
        );

        assert_eq!(adr_sut.title, "ADR-MVA-507 Decide about ...");
        assert_eq!(adr_sut.base_path, "base_path");
        assert_eq!(adr_sut.file_path, "a_path");
        assert_eq!(adr_sut.content, content.to_string());
        assert_eq!(adr_sut.tags, "");
    }

    #[test]
    fn test_update_date() {
        let content = "
        == ADR-MVA-507 Decide about ...

        *Status:* {wip}  *Date:* 2019-10-28
        ....";

        let mut adr_sut = super::Adr::from(
            "base_path".to_string(),
            "a_path".to_string(),
            content.to_string(),
        );

        assert_eq!(adr_sut.date, "2019-10-28");

        let date = Utc::today();
        adr_sut.update_date(date);

        let date = date.format("%Y-%m-%d");
        assert_eq!(adr_sut.date, date.to_string());

        let contain = format!("*Status:* {{wip}}  *Date:* {}", date);
        assert_eq!(true, adr_sut.content.contains(contain.as_str()));
    }

    #[test]
    fn test_update_title() {
        let content = "
        == ADR-MVA-507 Decide about ...

        *Status:* {wip}  *Date:* 2019-10-28
        ....";

        let mut adr_sut = super::Adr::from(
            "base_path".to_string(),
            "a_path".to_string(),
            content.to_string(),
        );

        assert_eq!(adr_sut.title, "ADR-MVA-507 Decide about ...");
        adr_sut.update_title("This is a new completly amazing title");

        assert_eq!(adr_sut.title, "This is a new completly amazing title");
        assert_eq!(
            true,
            adr_sut
                .content
                .contains("== This is a new completly amazing title")
        );
    }

    #[test]
    fn test_split_path() {
        let base_path = Path::new("/tmp/adr-samples/src");
        let file_path = Path::new("/tmp/adr-samples/src/my-decision.adoc");

        let values = split_path(base_path, file_path);
        assert_eq!(Path::new("/tmp/adr-samples/src"), values.0);
        assert_eq!(Path::new("my-decision.adoc"), values.1);

        //
        let base_path = Path::new("/tmp/adr-samples/src");
        let file_path = Path::new("/tmp/adr-samples/src/sub-folder/dir/my-decision.adoc");

        let values = split_path(base_path, file_path);
        assert_eq!(Path::new("/tmp/adr-samples/src"), values.0);
        assert_eq!(Path::new("sub-folder/dir/my-decision.adoc"), values.1);
        //
        let base_path = Path::new("/tmp/adr-samples/src");
        let file_path = Path::new("/another-folder/sub-folder/dir/my-decision.adoc");

        let values = split_path(base_path, file_path);
        assert_eq!(Path::new("/tmp/adr-samples/src"), values.0);
        assert_eq!(
            Path::new("/another-folder/sub-folder/dir/my-decision.adoc"),
            values.1
        );
    }

    #[test]
    fn test_transition_status_from_str() {
        assert_eq!(
            TransitionStatus::DECIDED,
            TransitionStatus::from_str(String::from("decided"))
        );
        assert_eq!(
            TransitionStatus::COMPLETED,
            TransitionStatus::from_str(String::from("completed"))
        );
        assert_eq!(
            TransitionStatus::COMPLETES,
            TransitionStatus::from_str(String::from("completes"))
        );
        assert_eq!(
            TransitionStatus::SUPERSEDED,
            TransitionStatus::from_str(String::from("superseded"))
        );
        assert_eq!(
            TransitionStatus::SUPERSEDES,
            TransitionStatus::from_str(String::from("supersedes"))
        );
        assert_eq!(
            TransitionStatus::CANCELLED,
            TransitionStatus::from_str(String::from("cancelled"))
        );
        assert_eq!(
            TransitionStatus::NONE,
            TransitionStatus::from_str(String::from("N/A"))
        );
    }

    #[test]
    fn test_transition_status_revert() {
        assert_eq!(
            TransitionStatus::COMPLETES,
            TransitionStatus::revert(TransitionStatus::COMPLETED)
        );
        assert_eq!(
            TransitionStatus::COMPLETED,
            TransitionStatus::revert(TransitionStatus::COMPLETES)
        );
        assert_eq!(
            TransitionStatus::SUPERSEDES,
            TransitionStatus::revert(TransitionStatus::SUPERSEDED)
        );
        assert_eq!(
            TransitionStatus::SUPERSEDED,
            TransitionStatus::revert(TransitionStatus::SUPERSEDES)
        );
    }
}
