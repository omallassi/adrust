extern crate slog;
extern crate slog_term;
use slog::*;

use std::fs::{self};
use std::io::{self};
use std::path::Path;
use std::collections::HashMap;

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
                Ok(_val) => debug!(get_logger(), "Copy template file from [{:?}] to [{:?}]", &path_to_template, &target_path), 
                Err(_why) => error!(get_logger(), "Unable to copy template from [{:?}] to [{:?}]", &path_to_template, &target_path), 
            };
            //build the Adr (and force the parsing)
            let newly_adr = match build_adr(Path::new(&cfg.adr_src_dir), &target_path) {
                Ok(adr) => adr,
                Err(why) => {
                    error!(get_logger(), "Got error [{:?}] while getting ADR [{:?}]", why, target_path);
                    panic!();
                },
            };

            let newly_adr = newly_adr.update_title(title);

            debug!(get_logger(), "Want to create ADR {:?}", &target_path);
            match fs::write(&target_path, newly_adr.content) {
                Ok(_val) => info!(get_logger(), "New ADR [{:?}] created", target_path), 
                Err(why) => {
                    error!(get_logger(), "Unable to create ADR [{:?}] - error [{:?}]", target_path, why);
                }
            };
            //
            
        }
        else {
            error!(get_logger(), "[{}] was not found", path_to_template.to_string_lossy());
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
        static ref RE: Regex = Regex::new(r"(\d+)-{1}").unwrap();
    }

    let mut id: usize = 0;
    if let Some(cap) = RE.captures(name) {
        debug!(get_logger(), "found first match [{}]", cap[1].to_string());
        id = cap[1].to_string().parse().unwrap();
    }
    else {
        debug!(get_logger(), "Unable to extract_seq_id from [{}]", name);
    }

    Ok(id)
}

fn get_last_seq_id(dir: &Path) -> usize {
    let adrs: Vec<Adr> = list_all_adr(dir).unwrap();
    let adrs_paths = adrs.iter().map(|adr| {
                                    let path = adr.path().clone();
                                    path
                                }).collect::<Vec<String>>();

    get_seq_id_from_name(&adrs_paths[0]).unwrap()
}


fn format_decision_name(cfg: AdrToolConfig, name: &str) -> Result<String> {
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

pub fn get_tags_popularity(base_path: &Path) -> Result<HashMap<String, u32>> {
    let mut popularity: HashMap<String, u32> = HashMap::new();
    for adr in list_all_adr(base_path)? {
            for tag in adr.tags_array.iter() {
                popularity.entry(tag.to_string()).and_modify(|e| { *e += 1 }).or_insert(1);
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
                    },
                    Err(_why) => {
                        debug!(get_logger(), "Unable to read file [{:?}]", entry.path());
                    }
                };
            }
        }
    }

    results.sort_by(|a, b| b.title.cmp(&a.title));

    Ok(results)
}

/// Given a complete `file_path`, returns the difference compared to `base_path`. 
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
pub fn split_path<'a>(base_path: &'a Path, file_path: &'a Path) -> (&'a Path, &'a Path) {
    debug!(get_logger(), "Want to split_path[{:?}] and [{:?}] ", base_path, file_path);
    match file_path.starts_with(base_path) {
        true => {
            (base_path, file_path.strip_prefix(base_path).unwrap_or(file_path))
        },
        false => {
            (base_path, file_path)
        },
    }
}

/// Build an ADR object given the provided arguments. Inside the ADR struct `file_path` will be splitted into `file_name` and `base_path`
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
pub fn build_adr(base_path: &Path, file_path: &Path) -> io::Result<Adr> {
    debug!(get_logger(), "Want to create ADR from [{}] ", file_path.display());
    let content = fs::read_to_string(file_path) ? ;

    //build the adr
    let splitted_file_path = split_path(base_path, file_path);
    let adr = Adr::from(String::from(splitted_file_path.0.to_str().unwrap()), String::from(splitted_file_path.1.to_str().unwrap()), content);
       
    Ok(adr)
}

pub fn transition_to_decided(base_path: &Path, file_name: &str) -> io::Result<bool> {
    transition_to(TransitionStatus::DECIDED, base_path, file_name, "")
}

pub fn transition_to_superseded_by(base_path: &Path, file_name: &str, by: &str) -> io::Result<bool> {
    transition_to(TransitionStatus::SUPERSEDED, base_path, file_name, by)
}

pub fn transition_to_completed_by(base_path: &Path, file_name: &str, by: &str) -> io::Result<bool> {
    transition_to(TransitionStatus::COMPLETED, base_path, file_name, by)
}

pub fn transition_to_obsoleted(base_path: &Path, file_name: &str) -> io::Result<bool> {
    transition_to(TransitionStatus::CANCELLED, base_path, file_name, "")
}

pub fn transition_to(transition: TransitionStatus, base_path: &Path, from: &str, by: &str) -> io::Result<bool> {
    let adr_from = match build_adr(base_path, Path::new(from)){
        Ok(adr) => adr,
        Err(why) => {
            error!(get_logger(), "Got error [{:?}] while getting ADR [{}]", why, from);
            panic!();
        },
    };

    let updated_adr_from_tuple = adr_from.update_status(transition);

    //if transition has been declined, we can stop here
    match updated_adr_from_tuple.1 {
        true => {
            debug!(get_logger(), "ADR [{}] has a new status [{}]", updated_adr_from_tuple.0.path().as_str(), updated_adr_from_tuple.0.status.as_str());
            match by.is_empty() {
                true => {
                    let content = &updated_adr_from_tuple.0.content;
                    fs::write(from, content)?;
        
                    info!(get_logger(), 
                        "Transitioned [{}] from [{}] to [{}]", 
                        updated_adr_from_tuple.0.path().as_str(), adr_from.status.as_str(), updated_adr_from_tuple.0.status.as_str());
        
                    Ok(updated_adr_from_tuple.1)
                }
                false => {
                    let adr_by = build_adr(base_path, Path::new(by))?;
                    let updated_adr_by_tuple = adr_by.update_status(TransitionStatus::revert(transition));

                    //if status has been accepted
                    match updated_adr_by_tuple.1 {
                        true => {
                            let updated_adr_from = updated_adr_from_tuple.0.add_reference(format!("{}", updated_adr_by_tuple.0.file_name).as_str());
                            let updated_adr_by = updated_adr_by_tuple.0.add_reference(format!("{}", updated_adr_from.file_name).as_str());
                
                            fs::write(from, updated_adr_from.content)?;
                            fs::write(by, updated_adr_by.content)?;
                
                            info!(get_logger(), 
                                "Transitioned [{}] from [{}] to [{}]", 
                                from, adr_from.status.as_str(), updated_adr_from_tuple.0.status.as_str());
                            info!(get_logger(), 
                                "Transitioned [{}] from [{}] to [{}]", 
                                by, adr_by.status.as_str(), updated_adr_by_tuple.0.status.as_str());
                
                            Ok(true)
                        }
                        false => {
                            error!(get_logger(), "ADR [{}] cannot be transitioned to [{:?}] - Status of [{:?}] is not [{:?}]", from, transition, by, TransitionStatus::DECIDED);
                            Ok(false)
                        }
                    }
                }
            }
        }
        false => {
            error!(get_logger(), "ADR [{}] cannot be transitioned to [{:?}]", from, transition);
            Ok(false)
        }
    }
}

#[derive(Debug, Default)]
pub struct Adr {
    //pub path: String, //the path from config.adr_root_dir (which is user dependant)
    pub file_name: String, 
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

    pub fn from(base_path: String, file_name: String, content: String) -> Adr {
        //get the title
        lazy_static! {
            static ref RE: Regex = Regex::new(r"= (.+)").unwrap();
        }
        let val = String::from(&content);
        let cap = match RE.captures(&val) {
            Some(val) => val[1].to_string(),
            None => {
                error!(get_logger(), "Unable to get title from base_path [{}] and file_name [{}]", base_path, file_name);
                "None".to_string()
            }
        };

        //build the tags
        let tags = Adr::get_tags(&val);

        //build the status
        lazy_static! {
            static ref RE_STATUS: Regex = Regex::new(r"\{(.+)\}").unwrap();
        }
        let status = match RE_STATUS.captures(&val) {
            Some(val) => val[1].trim().to_string(),
            None => {
                debug!(get_logger(), "Unable to get status from base_path [{}] and file_name [{}]", base_path, file_name);
                "None".to_string()
            }
        };

        //get date  
        lazy_static! {
            static ref RE_DATE: Regex = Regex::new(r"([0-9]{4}-[0-9]{2}-[0-9]{2})").unwrap();
        }
        let date = match RE_DATE.captures(&val) {
            Some(val) => val[1].trim().to_string(),
            None => {
                debug!(get_logger(), "Unable to get date from base_path [{}] and file_name [{}]", base_path, file_name);
                "None".to_string()
            }
        };

        //build the returned object
        let adr: Adr = Adr {
            file_name: file_name,
            base_path: base_path,
            content: content,
            title: cap,
            date: date,
            tags: tags.0,
            tags_array: tags.1,
            status: Status::from_str(status.clone()),
            state: AdrState { status: Status::from_str(status.clone()) },
        };

        adr
    }

    pub fn path(&self) -> String {
        let full_path = Path::new(self.base_path.as_str()).join(self.file_name.as_str());

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

        let tags = tags_str.split('#').filter(|s| s.len() > 0).map(|s| s.to_string()).collect();

        (tags_str, tags)
    }

    pub fn update_status(&self, transition: TransitionStatus) -> (Adr, bool) {
        let current_status = format!("{{{status}}}", status = self.status.as_str() ); //you escape { with a { and final status is {wip}  o_O
        let mut state = self.state;
        let has_been_modified = state.transition(transition);

        debug!(get_logger(), "Want transition [{:?}] - Adr State transitioned from [{:?}] to [{:?}] - has been modified [{:?}]", transition, self.state, state, has_been_modified);
        
        if has_been_modified {
            let new_status = format!("{{{status}}}", status = state.status.as_str() );
            debug!(get_logger(), "Transitioned to [{}]", state.status.as_str());
            let new_content = self.content.replace(current_status.as_str(), new_status.as_str());

            let returned_adr = Adr {
                    file_name: String::from(self.file_name.as_str()),
                    base_path: String::from(self.base_path.as_str()),
                    content: new_content,
                    title: String::from(self.title.as_str()),
                    date: String::from(self.date.as_str()),
                    tags: String::from(self.tags.as_str()),
                    tags_array: self.tags_array.clone(),
                    status: state.status,
                    state: state,
                };

            let returned_adr = returned_adr.update_date(Utc::today());

            //Todo maybe I it would be better to implement Copy Trait
            (returned_adr, has_been_modified)
        }
        else {
            debug!(get_logger(), "Transition has been declined");
            (self.clone(), false)
        }
    }

    pub fn add_reference(&self, adr_title: &str) -> Adr {
        let current_status = format!("{{{status}}}", status = self.status.as_str() ); //you escape { with a { and final status is {wip}  o_O
        let new_status = format!("{updated_by} {by}", updated_by = current_status.as_str(), by = adr_title);

        debug!(get_logger(), "Want to add reference - current status [{:?}] - new status [{:?}]", current_status, new_status);

        let new_content = self.content.replace(current_status.as_str(), new_status.as_str());
        Adr {
            file_name: String::from(self.file_name.as_str()),
            base_path: String::from(self.base_path.as_str()),
            content: new_content,
            title: String::from(self.title.as_str()),
            date: String::from(self.date.as_str()),
            tags: String::from(self.tags.as_str()),
            tags_array: self.tags_array.clone(),
            status: self.status.clone(),
            state: self.state.clone(),
        }
    }

    pub fn update_date(&self, today: Date<Utc>) -> Adr {
        let new_date = today.format("%Y-%m-%d").to_string();

        debug!(get_logger(), "Want to update ADR to date [{}]", new_date);

        let re = Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap();
        let new_content = re.replace(self.content.as_str(), new_date.as_str()).as_ref().to_owned();

        Adr {
            file_name: String::from(self.file_name.as_str()),
            base_path: String::from(self.base_path.as_str()),
            content: new_content,
            title: String::from(self.title.as_str()),
            date: String::from(new_date.as_str()),
            tags: String::from(self.tags.as_str()),
            tags_array: self.tags_array.clone(),
            status: self.status.clone(),
            state: self.state.clone(),
        }
    }

    pub fn update_title(&self, title: &str) -> Adr {
        let mut adoc_title = String::from("");
        adoc_title.push_str(&title);

        let new_content = &self.content;
        let new_content = new_content.replacen(self.title.as_str(), adoc_title.as_str(), 1);

        Adr {
            file_name: String::from(self.file_name.as_str()),
            base_path: String::from(self.base_path.as_str()),
            content: new_content,
            title: String::from(title),
            date: String::from(self.date.as_str()),
            tags: String::from(self.tags.as_str()),
            tags_array: self.tags_array.clone(),
            status: self.status.clone(),
            state: self.state.clone(),
        }
    }
}

impl Clone for Adr {
    fn clone(&self) -> Adr {
        Adr {
            file_name: String::from(self.file_name.as_str()),
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
    fn default() -> Self { Status::WIP }
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
    fn default() -> Self { AdrState { status: Status::WIP } }
}

impl State for AdrState {

    fn transition(&mut self, transition: TransitionStatus) -> bool {  
        let current_state = self.status.as_str();
        let current_status = &self.status;

        let mut has_been_modified = true;
        
        let next_status= match current_status {
            Status::WIP => {
                match transition {
                    TransitionStatus::DECIDED => {
                        self.status = Status::DECIDED;
                        Status::DECIDED
                    },
                    TransitionStatus::CANCELLED => {
                        self.status = Status::CANCELLED;
                        Status::CANCELLED
                    },
                    _ => {
                        has_been_modified = false;
                        Status::WIP
                    }
                }
            },
            Status::DECIDED => {
                match transition {
                    TransitionStatus::COMPLETED => {
                        self.status = Status::COMPLETED;
                        Status::COMPLETED
                    },
                    TransitionStatus::COMPLETES => {
                        self.status = Status::COMPLETES;
                        Status::COMPLETES
                    },
                    TransitionStatus::CANCELLED => {
                        self.status = Status::CANCELLED;
                        Status::CANCELLED
                    },
                    TransitionStatus::SUPERSEDED => {
                        self.status = Status::SUPERSEDED;
                        Status::SUPERSEDED
                    },
                    TransitionStatus::SUPERSEDES => {
                        self.status = Status::SUPERSEDES;
                        Status::SUPERSEDES
                    },
                    _ => {
                        has_been_modified = false;
                        Status::DECIDED
                    }
                }
            },
            Status::COMPLETED => {
                match transition {
                    TransitionStatus::SUPERSEDED => {
                        self.status = Status::SUPERSEDED;
                        Status::SUPERSEDED
                    },
                    TransitionStatus::CANCELLED => {
                        self.status = Status::CANCELLED;
                        Status::CANCELLED
                    },
                    _ => {
                        has_been_modified = false;
                        Status::COMPLETED
                    }
                }
            },
            Status::COMPLETES => {
                match transition {
                    TransitionStatus::CANCELLED => {
                        self.status = Status::CANCELLED;
                        Status::CANCELLED
                    },
                    TransitionStatus::SUPERSEDED => {
                        self.status = Status::SUPERSEDED;
                        Status::SUPERSEDED
                    },
                    _ => {
                        has_been_modified = false;
                        Status::COMPLETES
                    }
                }
            },
            Status::SUPERSEDED => {
                match transition {
                    TransitionStatus::CANCELLED => {
                        self.status = Status::CANCELLED;
                        Status::CANCELLED
                    },
                    _ => {
                        has_been_modified = false;
                        Status::SUPERSEDED
                    }
                }
            },
            Status::SUPERSEDES => {
                match transition {
                    TransitionStatus::CANCELLED => {
                        self.status = Status::CANCELLED;
                        Status::CANCELLED
                    },
                    _ => {
                        has_been_modified = false;
                        Status::SUPERSEDES
                    }
                }
            },
            Status::CANCELLED => {
                match transition {
                    _ => {
                        has_been_modified = false;
                        Status::CANCELLED
                    }
                }
            },

            _ => {
                has_been_modified = false;
                Status::NONE
            },
        };

        self.status = next_status;
        debug!(get_logger(), "transition [{:?}] has been called from [{:?}] to [{:?}]", transition, current_state, self.status);

        has_been_modified
    }

    fn build(status: Status) -> AdrState {
        AdrState {
            status: status,
        } 
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

    use crate::adr_repo::{*};

    #[test]
    fn test_adr_update_status() {
        let sut = Adr {
            base_path: String::from("/tmp/n"),
            file_name: String::from("/a"),
            content: String::from("== ADR-MVA-507 Decide about ...\n\n*Status:* {wip} *Date:* 2019-10-28\n\n[cols=\",\",options=..."),
            title: String::from("String::from(self.title.as_str())"),
            date: String::from("2023-10-28"),
            tags: String::from(""),
            tags_array: Vec::new(),
            status: Status::WIP,
            state: AdrState { status: Status::WIP },
        };

        let new_adr = sut.update_status(TransitionStatus::DECIDED);

        assert_eq!(new_adr.0.status, Status::DECIDED);
        assert_eq!(new_adr.0.state, AdrState { status: Status::DECIDED } );
        assert_eq!(new_adr.0.content.contains(Status::DECIDED.as_str()), true);
    }

    #[test]
    fn test_adr_add_reference() {
            let sut = Adr {
                base_path: String::from("/tmp/n"),
                file_name: String::from("/a"),
                content: String::from("== ADR-MVA-507 Decide about ...\n\n*Status:* {decided} *Date:* 2019-10-28\n\n[cols=\",\",options=\"header\",%autowidth]\n|===\n|role ....."),
                title: String::from("String::from(self.title.as_str())"),
                date: String::from("2023-10-28"),
                tags: String::from(""),
                tags_array: Vec::new(),
                status: Status::DECIDED,
                state: AdrState { status: Status::DECIDED },
            };

            let new_adr = sut.add_reference("by adr-num-123");

            assert_eq!(new_adr.status, Status::DECIDED);
            assert_eq!(new_adr.state, AdrState { status: Status::DECIDED } );
            
            let expected_status = "{decided} by adr-num-123 *Date:* 2019-10-28";
            assert_eq!(new_adr.content.contains(expected_status), true);
    }

    #[test]
    fn test_state_machine() {
        let mut state = super::AdrState::build(super::Status::WIP);
        assert_eq!(state, super::AdrState { status: super::Status::WIP});
        state.transition(super::TransitionStatus::COMPLETED);
        assert_eq!(state, super::AdrState { status: super::Status::WIP});
        state.transition(super::TransitionStatus::DECIDED);
        assert_eq!(state, super::AdrState { status: super::Status::DECIDED});
        state.transition(super::TransitionStatus::SUPERSEDED);
        assert_eq!(state, super::AdrState { status: super::Status::SUPERSEDED});
        state.transition(super::TransitionStatus::CANCELLED);
        assert_eq!(state, super::AdrState { status: super::Status::CANCELLED});
    }

    #[test]
    fn test_state_machine_2() {
        let mut state = super::AdrState::build(super::Status::WIP);
        assert_eq!(state, super::AdrState { status: super::Status::WIP});
        state.transition(super::TransitionStatus::SUPERSEDED);
        assert_eq!(state, super::AdrState { status: super::Status::WIP});
        state.transition(super::TransitionStatus::DECIDED);
        assert_eq!(state, super::AdrState { status: super::Status::DECIDED});
        state.transition(super::TransitionStatus::COMPLETED);
        assert_eq!(state, super::AdrState { status: super::Status::COMPLETED});
        state.transition(super::TransitionStatus::SUPERSEDED);
        assert_eq!(state, super::AdrState { status: super::Status::SUPERSEDED});
        state.transition(super::TransitionStatus::CANCELLED);
        assert_eq!(state, super::AdrState { status: super::Status::CANCELLED});
    }

    #[test]
    fn test_state_machine_wip_to_cancelled() {
        let mut state = super::AdrState::build(super::Status::WIP);
        assert_eq!(state, super::AdrState { status: super::Status::WIP});
        state.transition(super::TransitionStatus::CANCELLED);
        assert_eq!(state, super::AdrState { status: super::Status::CANCELLED});
    }

    #[test]
    fn test_state_machine_decided_to_cancelled() {
        let mut state = super::AdrState::build(super::Status::DECIDED);
        assert_eq!(state, super::AdrState { status: super::Status::DECIDED});
        state.transition(super::TransitionStatus::CANCELLED);
        assert_eq!(state, super::AdrState { status: super::Status::CANCELLED});
    }

    #[test]
    fn test_state_machine_decided_to_completes() {
        let mut state = super::AdrState::build(super::Status::DECIDED);
        assert_eq!(state, super::AdrState { status: super::Status::DECIDED});
        state.transition(super::TransitionStatus::COMPLETES);
        assert_eq!(state, super::AdrState { status: super::Status::COMPLETES});
    }

    #[test]
    fn test_state_machine_decided_to_supersedes() {
        let mut state = super::AdrState::build(super::Status::DECIDED);
        assert_eq!(state, super::AdrState { status: super::Status::DECIDED});
        state.transition(super::TransitionStatus::SUPERSEDES);
        assert_eq!(state, super::AdrState { status: super::Status::SUPERSEDES});
    }

    #[test]
    fn test_state_machine_decided_to_fail() {
        let mut state = super::AdrState::build(super::Status::DECIDED);
        assert_eq!(state, super::AdrState { status: super::Status::DECIDED});
        state.transition(super::TransitionStatus::NONE);
        assert_eq!(state, super::AdrState { status: super::Status::DECIDED});
    }

    #[test]
    fn test_state_machine_completed_to_cancelled() {
        let mut state = super::AdrState::build(super::Status::COMPLETED);
        assert_eq!(state, super::AdrState { status: super::Status::COMPLETED});
        state.transition(super::TransitionStatus::CANCELLED);
        assert_eq!(state, super::AdrState { status: super::Status::CANCELLED});
    }

    #[test]
    fn test_state_machine_superseded_to_fail() {
        let mut state = super::AdrState::build(super::Status::SUPERSEDED);
        assert_eq!(state, super::AdrState { status: super::Status::SUPERSEDED});
        state.transition(super::TransitionStatus::DECIDED);
        assert_eq!(state, super::AdrState { status: super::Status::SUPERSEDED});
    }

    #[test]
    fn test_state_machine_cancelled_to_cancelled() {
        let mut state = super::AdrState::build(super::Status::CANCELLED);
        assert_eq!(state, super::AdrState { status: super::Status::CANCELLED});
        state.transition(super::TransitionStatus::DECIDED);
        assert_eq!(state, super::AdrState { status: super::Status::CANCELLED});
    }

    #[test]
    fn test_get_seq() {
        let seq = super::get_seq_id_from_name("01-my-decision.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq = super::get_seq_id_from_name("00000010-my-decision.adoc").unwrap();
        assert_eq!(seq, 10);
        let seq = super::get_seq_id_from_name("mypath/00000001-my-decision.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq = super::get_seq_id_from_name("mypath/00000001-my-decision-594.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq = super::get_seq_id_from_name("mypath/00000001-my-decision-594-full.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq = super::get_seq_id_from_name("00000001-my-decision-594-full.adoc").unwrap();
        assert_eq!(seq, 1);
        let seq =
            super::get_seq_id_from_name("mypath/00000001/00000002-my-decision-594-full.adoc").unwrap();
        assert_eq!(seq, 2
        );

        let seq =
        super::get_seq_id_from_name("path/my-decision-full.adoc").unwrap();
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

        let seq = super::get_last_seq_id_from_all(paths);
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

        let seq = super::get_last_seq_id_from_all(paths);
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

        let adr_sut = super::Adr::from("base_path".to_string(), "a_path".to_string(), content.to_string());

        assert_eq!(adr_sut.title, "ADR-MVA-507 Decide about ...");
        assert_eq!(adr_sut.date, "2019-10-28");
        assert_eq!(adr_sut.base_path, "base_path");
        assert_eq!(adr_sut.file_name, "a_path");
        assert_eq!(adr_sut.content, content.to_string());
        assert_eq!(adr_sut.tags, "#deployment view #network #security ");
        assert_eq!(adr_sut.status, super::Status::WIP);
    }


    #[test]
    fn test_build_adr(){
        let env = match TempDir::new("my_temp_folder") {
            Ok(env) => env, 
            Err(why) => {
                println!("Unable to get env dir [{}]", why);
                panic!(why);
            }
        };

        let content = "// Include contents of docinfo.html
        :docinfo1:
        :wip: pass:quotes[[.label.wip]#In Progress#]
        :decided: pass:q[[.label.decided]#Decided#]
        :completed: pass:q[[.label.updated]#Completed By#]
        :completes: pass:q[[.label.updated]#Completes#]
        :supersedes: pass:q[[.label.updated]#Supersedes#]
        :superseded: pass:q[[.label.obsoleted]#Superseded By#]
        :obsoleted: pass:q[[.label.obsoleted]#Obsolete#]
        
        == ADR-WIP a wip decision
        
        *Status:* {decided}  *Date:* 2019-10-28
        
        === Context and Problem Statement
        ......";

        let to = PathBuf::from(env.path()).join("decided.adoc");
        fs::write(to.as_path(), content).unwrap();

        println!("Want to work with [{}]", to.display());

        let adr = super::build_adr(env.path(), to.as_path()).unwrap();
        assert_eq!(Status::DECIDED, adr.status);
        assert_eq!("ADR-WIP a wip decision", adr.title);
        assert_eq!("2019-10-28", adr.date);
        assert_eq!(format!("{}", env.path().display()), adr.base_path);
        assert_eq!("decided.adoc", adr.file_name);
        assert_eq!("", adr.tags);
    }

    #[test]
    fn test_create_adr_wo_prefix(){
        let env = match TempDir::new("my_temp_folder") {
            Ok(env) => {
                println!("Working with env dir [{}]", env.path().display());
                env
            }, 
            Err(why) => {
                println!("Unable to get env dir [{}]", why);
                panic!(why);
            }
        };
        //set config
        let config = AdrToolConfig {
            log_level: 6,
            //adr_root_dir: format!("{}", env.path().display()),
            adr_src_dir: format!("{}", env.path().display()),
            adr_template_dir: format!("{}", env.path().display()),
            adr_template_file: String::from("template.adoc"),
            adr_search_index: format!("{}", env.path().display()),
            use_id_prefix: false,
            id_prefix_width: 3,
        };

        //set template
        let template = ":docinfo1:
        :wip: pass:quotes[[.label.wip]#In Progress#]
        :decided: pass:q[[.label.decided]#Decided#]
        :completed: pass:q[[.label.updated]#Completed By#]
        :completes: pass:q[[.label.updated]#Completes#]
        :supersedes: pass:q[[.label.updated]#Supersedes#]
        :superseded: pass:q[[.label.obsoleted]#Superseded By#]
        :obsoleted: pass:q[[.label.obsoleted]#Obsolete#]
        
        = short title of solved problem and solution
        
        *Status:* {wip} *Date:* 2019-10-28
        ...";
        let to = PathBuf::from(env.path()).join("template.adoc");
        fs::write(to.as_path(), template).unwrap();

        //test
        let created = super::create_adr(config, "title of the ADR");
        //
        assert!(created.unwrap());
        assert_eq!(true, env.path().exists());
        assert_eq!(true, env.path().join("title-of-the-adr.adoc").exists());
    }

    #[test]
    fn test_create_adr_w_prefix(){
        let env = match TempDir::new("my_temp_folder") {
            Ok(env) => {
                println!("Working with env dir [{}]", env.path().display());
                env
            }, 
            Err(why) => {
                println!("Unable to get env dir [{}]", why);
                panic!(why);
            }
        };
        //set config
        let config = AdrToolConfig {
            log_level: 6,
            //adr_root_dir: format!("{}", env.path().display()),
            adr_src_dir: format!("{}", env.path().display()),
            adr_template_dir: format!("{}", env.path().display()),
            adr_template_file: String::from("template.adoc"),
            adr_search_index: format!("{}", env.path().display()),
            use_id_prefix: true,
            id_prefix_width: 3,
        };

        //set template
        let template = ":docinfo1:
        :wip: pass:quotes[[.label.wip]#In Progress#]
        :decided: pass:q[[.label.decided]#Decided#]
        :completed: pass:q[[.label.updated]#Completed By#]
        :completes: pass:q[[.label.updated]#Completes#]
        :supersedes: pass:q[[.label.updated]#Supersedes#]
        :superseded: pass:q[[.label.obsoleted]#Superseded By#]
        :obsoleted: pass:q[[.label.obsoleted]#Obsolete#]
        
        = short title of solved problem and solution
        
        *Status:* {wip} *Date:* 2019-10-28
        ...";
        let to = PathBuf::from(env.path()).join("template.adoc");
        fs::write(to.as_path(), template).unwrap();

        //set a couple of already present files 
        let to = PathBuf::from(env.path()).join("001-ADR-1.adoc");
        fs::write(to.as_path(), template).unwrap();
        let to = PathBuf::from(env.path()).join("003-ADR-2.adoc");
        fs::write(to.as_path(), template).unwrap();

        //test
        let created = super::create_adr(config, "title of the ADR");
        //
        assert!(created.unwrap());
        assert_eq!(true, env.path().exists());
        assert_eq!(true, env.path().join("004-title-of-the-adr.adoc").exists());
    }

    #[test]
    fn test_get_tags_popularity(){
        let env = match TempDir::new("my_temp_folder") {
            Ok(env) => {
                println!("Working with env dir [{}]", env.path().display());
                env
            }, 
            Err(why) => {
                println!("Unable to get env dir [{}]", why);
                panic!(why);
            }
        };
        //set config
        let config = AdrToolConfig {
            log_level: 6,
            //adr_root_dir: format!("{}", env.path().display()),
            adr_src_dir: format!("{}", env.path().display()),
            adr_template_dir: format!("{}", env.path().display()),
            adr_template_file: String::from("template.adoc"),
            adr_search_index: format!("{}", env.path().display()),
            use_id_prefix: true,
            id_prefix_width: 3,
        };

        //set template
        let template = ":docinfo1:
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
        //set a couple of already present files 
        let to = PathBuf::from(env.path()).join("001-ADR-1.adoc");
        fs::write(to.as_path(), template).unwrap();
        let to = PathBuf::from(env.path()).join("003-ADR-2.adoc");
        fs::write(to.as_path(), template).unwrap();
        let to = PathBuf::from(env.path()).join("004-ADR-2.adoc");
        fs::write(to.as_path(), template).unwrap();

        //test
        let tags = super::get_tags_popularity(env.path()).unwrap();
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

        let adr_sut = super::Adr::from("base_path".to_string(), "a_path".to_string(), content.to_string());

        assert_eq!(adr_sut.title, "ADR-MVA-507 Decide about ...");
        assert_eq!(adr_sut.base_path, "base_path");
        assert_eq!(adr_sut.file_name, "a_path");
        assert_eq!(adr_sut.content, content.to_string());
        assert_eq!(adr_sut.tags, "");
    }

    #[test]
    fn test_update_date() {
        let content = "
        == ADR-MVA-507 Decide about ...
        
        *Status:* {wip}  *Date:* 2019-10-28
        ....";

        let adr_sut = super::Adr::from("base_path".to_string(), "a_path".to_string(), content.to_string());

        assert_eq!(adr_sut.date, "2019-10-28");

        let date = Utc::today();
        let adr_sut = adr_sut.update_date(date);

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

        let adr_sut = super::Adr::from("base_path".to_string(), "a_path".to_string(), content.to_string());

        assert_eq!(adr_sut.title, "ADR-MVA-507 Decide about ...");
        let adr_sut = adr_sut.update_title("This is a new completly amazing title");

        assert_eq!(adr_sut.title, "This is a new completly amazing title");
        assert_eq!(true, adr_sut.content.contains("== This is a new completly amazing title"));
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
        assert_eq!(Path::new("/another-folder/sub-folder/dir/my-decision.adoc"), values.1);
    }

    #[test]
    fn test_transition_status_from_str(){
        assert_eq!(TransitionStatus::DECIDED, TransitionStatus::from_str(String::from("decided")));
        assert_eq!(TransitionStatus::COMPLETED, TransitionStatus::from_str(String::from("completed")));
        assert_eq!(TransitionStatus::COMPLETES, TransitionStatus::from_str(String::from("completes")));
        assert_eq!(TransitionStatus::SUPERSEDED, TransitionStatus::from_str(String::from("superseded")));
        assert_eq!(TransitionStatus::SUPERSEDES, TransitionStatus::from_str(String::from("supersedes")));
        assert_eq!(TransitionStatus::CANCELLED, TransitionStatus::from_str(String::from("cancelled")));
        assert_eq!(TransitionStatus::NONE, TransitionStatus::from_str(String::from("N/A")));
    }

    #[test]
    fn test_transition_status_revert(){
        assert_eq!(TransitionStatus::COMPLETES, TransitionStatus::revert(TransitionStatus::COMPLETED));
        assert_eq!(TransitionStatus::COMPLETED, TransitionStatus::revert(TransitionStatus::COMPLETES));
        assert_eq!(TransitionStatus::SUPERSEDES, TransitionStatus::revert(TransitionStatus::SUPERSEDED));
        assert_eq!(TransitionStatus::SUPERSEDED, TransitionStatus::revert(TransitionStatus::SUPERSEDES));
    }
}
