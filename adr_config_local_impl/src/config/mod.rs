use std::fs;
extern crate slog;
extern crate slog_term;
use slog::*;

#[derive(Serialize, Deserialize)]
pub struct AdrToolConfig {
    pub log_level: usize,
    pub adr_root_dir: String,
    pub adr_src_dir: String,
    pub adr_template_dir: String,
    pub adr_search_index: String
}

impl ::std::default::Default for AdrToolConfig {
    fn default() -> Self {
        AdrToolConfig {
            adr_root_dir: "/tmp/adr-samples".to_string(),
            adr_src_dir: "/tmp/adr-samples/src".to_string(),
            adr_template_dir: "/tmp/adr-samples/templates".to_string(),
            adr_search_index: "/tmp/adr-samples/.index".to_string(),
            log_level: 4, //info
        }
    }
}

fn get_logger() -> slog::Logger {
    let cfg: AdrToolConfig = get_config();

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

pub fn init() -> Result<()> {
    let cfg: AdrToolConfig = get_config();
    let path = String::from(cfg.adr_root_dir);
    fs::create_dir_all(&path)?;
    info!(get_logger(), "[{}] created]", path);

    let path = String::from(cfg.adr_src_dir);
    fs::create_dir_all(&path)?;
    info!(get_logger(), "[{}] created]", path);

    let path = String::from(cfg.adr_template_dir);
    fs::create_dir_all(&path)?;
    info!(get_logger(), "[{}] created]", &path);

    let path = String::from(cfg.adr_search_index);
    fs::create_dir_all(&path)?;
    info!(get_logger(), "[{}] created]", &path);

    fs::copy(
        "./templates/adr-template-v0.1.adoc",
        format!("{}/adr-template-v0.1.adoc", &path),
    )?;

    Ok(())
}

pub fn set_config(name: &str, value: &str) -> Result<()> {
    let mut cfg: AdrToolConfig = get_config();
    if "adr_root_dir" == name {
        let mut adr_src_dir = String::from(value);
        adr_src_dir.push_str("/src");

        let mut adr_template_dir = String::from(value);
        adr_template_dir.push_str("/templates");

        let mut adr_search_index = String::from(value);
        adr_search_index.push_str("/.index");

        let new_cfg = AdrToolConfig {
            adr_root_dir: String::from(value),
            adr_src_dir: adr_src_dir,
            adr_template_dir: adr_template_dir,
            adr_search_index: adr_search_index,
            log_level: cfg.log_level, //info
        };

        confy::store("adrust-tools", new_cfg).unwrap();
    }
    if "log_level" == name {
        cfg.log_level = value.parse().unwrap();      
        confy::store("adrust-tools", cfg).unwrap();  
    }

    Ok(())
}

pub fn get_config() -> AdrToolConfig {
    let cfg: AdrToolConfig = match confy::load("adrust-tools") {
        Err(_why) => {
            AdrToolConfig::default()
        },
        Ok(e) => e,
    };

    cfg
}
