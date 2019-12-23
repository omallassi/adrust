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
    pub adr_template_file: String,
    pub adr_search_index: String
}

pub const LOG_LEVEL: &str = "log_level";
pub const ADR_ROOT_DIR: &str = "adr_root_dir";
pub const ADR_SRC_DIR: &str = "adr_src_dir";
pub const ADR_TEMPLATE_DIR: &str = "adr_template_dir";
pub const ADR_TEMPLATE_FILE: &str = "adr_template_file";
pub const ADR_SEARCH_INDEX: &str = "adr_search_dir";

impl ::std::default::Default for AdrToolConfig {
    fn default() -> Self {
        AdrToolConfig {
            adr_root_dir: "/tmp/adr-samples".to_string(),
            adr_src_dir: "/tmp/adr-samples/src".to_string(),
            adr_template_dir: "/tmp/adr-samples/templates".to_string(),
            adr_template_file: "adr-template-v0.1.adoc".to_string(),
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

    match fs::copy(
        "./templates/adr-template-v0.1.adoc",
        format!("{}/adr-template-v0.1.adoc", &path),
    ) {
        Err(_why) => {
            warn!(get_logger(), "Unable to create [{}]", format!("{}/adr-template-v0.1.adoc", &path));
        }
        Ok(_val) => {
            info!(get_logger(), "[{}] created]",format!("{}/adr-template-v0.1.adoc", &path));
        }
    };

    let path = String::from(cfg.adr_search_index);
    fs::create_dir_all(&path)?;
    info!(get_logger(), "[{}] created]", &path);

    Ok(())
}

pub fn set_config(name: &str, value: &str) -> Result<()> {
    set_config_from_name("adrust-tools", name, value)
}

pub fn get_config() -> AdrToolConfig {
    get_config_from_name("adrust-tools")
}

fn set_config_from_name(config: &str, name: &str, value: &str) -> Result<()> {
    if "adr_root_dir" == name {
        let cfg: AdrToolConfig = get_config();
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
            adr_template_file: cfg.adr_template_file,
            adr_search_index: adr_search_index,
            log_level: cfg.log_level, //info
        };

        confy::store(config, new_cfg).unwrap();
    }
    if "adr_template_file" == name {
        let mut cfg: AdrToolConfig = get_config();
        cfg.adr_template_file = String::from(value);      
        confy::store(config, cfg).unwrap();
    }
    if "log_level" == name {
        let mut cfg: AdrToolConfig = get_config();
        cfg.log_level = value.parse().unwrap();      
        confy::store(config, cfg).unwrap();
    }

    Ok(())
}

fn get_config_from_name(config: &str) -> AdrToolConfig {
    let cfg: AdrToolConfig = match confy::load(config) {
        Err(_why) => {
            warn!(get_logger(), "Returning default configuration file");
            AdrToolConfig::default()
        },
        Ok(e) => e,
    };

    cfg
}

#[cfg(test)]
mod tests {
    use directories::ProjectDirs;
    use std::fs::{self};

    #[test]
    fn test_set_config_log_level() {
        let config = "adrust-tools-4-tests";

        super::set_config_from_name(config, "log_level", "7").unwrap();
        let cfg = super::get_config_from_name(config);

        assert_eq!(cfg.log_level, 7);

        teardown(config);
    }

    fn teardown(name: &str) {
        //delete confy files
        if let Some(dir) = ProjectDirs::from("rs", name, name) {
            fs::remove_dir_all(dir.config_dir().to_str().unwrap()).unwrap();
        }
    }

    #[test]
    fn test_set_config_adr_root() {
        let config = "adrust-tools-4-tests-2";

        super::set_config_from_name(config, "adr_root_dir", "/tmp/adr-samples-4-tests").unwrap();
        let cfg = super::get_config_from_name(config);
        assert_eq!(cfg.adr_root_dir, "/tmp/adr-samples-4-tests");
        assert_eq!(cfg.adr_search_index, "/tmp/adr-samples-4-tests/.index");
        assert_eq!(cfg.adr_template_dir, "/tmp/adr-samples-4-tests/templates");
        assert_eq!(cfg.adr_template_file, "adr-template-v0.1.adoc");
        assert_eq!(cfg.log_level, 4);

        teardown(config);
    }

    #[test]
    fn test_set_config_adr_template_file() {
        let config = "adrust-tools-4-tests-3";

        super::set_config_from_name(config, "adr_template_file", "new-template.adoc").unwrap();
        let cfg = super::get_config_from_name(config);
        assert_eq!(cfg.adr_template_file, "new-template.adoc");

        teardown(config);
    }

}