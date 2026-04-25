use std::fs;
extern crate slog;
extern crate slog_term;
use slog::*;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdrToolConfig {
    pub log_level: usize,
    //pub adr_root_dir: String,
    pub adr_src_dir: String,
    pub adr_template_dir: String,
    pub adr_template_file: String,
    pub adr_search_index: String,
    pub use_id_prefix: bool,
    pub id_prefix_width: usize,
    pub ollama_url: String,
    pub ollama_model: String,
}

pub const LOG_LEVEL: &str = "log_level";
pub const ADR_ROOT_DIR: &str = "adr_root_dir";
pub const ADR_SRC_DIR: &str = "adr_src_dir";
pub const ADR_TEMPLATE_DIR: &str = "adr_template_dir";
pub const ADR_TEMPLATE_FILE: &str = "adr_template_file";
pub const ADR_SEARCH_INDEX: &str = "adr_search_dir";
pub const USE_ID_PREFIX: &str = "use_id_prefix";
pub const ID_PREFIX_WIDTH: &str = "id_prefix_width";
pub const OLLAMA_URL: &str = "ollama_url";
pub const OLLAMA_MODEL: &str = "ollama_model";

impl ::std::default::Default for AdrToolConfig {
    fn default() -> Self {
        AdrToolConfig {
            //adr_root_dir: "/tmp/adr-samples".to_string(),//irrelevant ? following murex convention, it seems more natural to keep adr_root_dir than adr_scr_dir (cf. adr_template_dir)
            adr_src_dir: "/tmp/adr-samples/src".to_string(), //"npryce convention :  doc/adr; murex convention : docs/adr"
            adr_template_dir: "/tmp/adr-samples/templates".to_string(), //"npryce convention : src; murex convention : docs/adr/templates"
            adr_template_file: "adr-template-v0.1.adoc".to_string(), //"npryce convention : template.md; murex convention : template.adoc"
            adr_search_index: "/tmp/adr-samples/.index".to_string(),
            log_level: 4, //info
            use_id_prefix: true,
            id_prefix_width: 6,
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "llama3.2".to_string(),
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

    slog::Logger::root(drain, o!())
}

pub fn init() -> Result<()> {
    init_from_name("adrust-tools")
}

pub fn init_from_name(config_name: &str) -> Result<()> {
    let cfg: AdrToolConfig = get_config_from_name(config_name);
    // let path = cfg.adr_root_dir;
    // fs::create_dir_all(&path)?;
    // info!(get_logger(), "[{}] created]", path);

    let path = cfg.adr_src_dir;
    fs::create_dir_all(&path)?;
    info!(get_logger(), "[{}] created]", path);

    let path = cfg.adr_template_dir;
    fs::create_dir_all(&path)?;
    info!(get_logger(), "[{}] created]", &path);

    let from = "./templates/adr-template-v0.1.adoc";
    //check 'from' and 'to' are not the same to avoid file to be truncated
    match fs::metadata(&from) {
        Ok(_) => {
            warn!(
                get_logger(),
                "File [{}] already exists, it will not be copied", &from
            );
        }
        Err(_) => {
            match fs::copy(&from, format!("{0}/{1}", &path, cfg.adr_template_file)) {
                Err(why) => {
                    warn!(
                        get_logger(),
                        "Unable to create [{}] - [{}]",
                        format!("{0}/{1}", &path, cfg.adr_template_file),
                        why
                    );
                }
                Ok(_val) => {
                    info!(
                        get_logger(),
                        "[{}] created]",
                        format!("{0}/{1}", &path, cfg.adr_template_file)
                    );
                }
            };
        }
    }

    let path = cfg.adr_search_index;
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

fn store_config(config: &str, name: &str, cfg: &AdrToolConfig) {
    if let Err(why) = confy::store(config, None, cfg) {
        error!(
            get_logger(),
            "Error while updating config file for property [{}] - [{}]", name, why
        );
    }
}

pub fn set_config_from_name(config: &str, name: &str, value: &str) -> Result<()> {
    if ADR_ROOT_DIR == name {
        // Applying standard convention: derive src, templates and index from root
        let cfg = get_config_from_name(config);
        let new_cfg = AdrToolConfig {
            adr_src_dir:      String::from(value),
            adr_template_dir: format!("{}", Path::new(value).join("templates").display()),
            adr_search_index: format!("{}", Path::new(value).join(".index").display()),
            ..cfg
        };
        confy::store(config, None, new_cfg).unwrap();
        return Ok(());
    }

    let mut cfg = get_config_from_name(config);
    match name {
        ADR_SRC_DIR       => cfg.adr_src_dir      = String::from(value),
        ADR_TEMPLATE_DIR  => cfg.adr_template_dir = String::from(value),
        ADR_TEMPLATE_FILE => cfg.adr_template_file = String::from(value),
        OLLAMA_URL        => cfg.ollama_url        = String::from(value),
        OLLAMA_MODEL      => cfg.ollama_model      = String::from(value),
        LOG_LEVEL         => cfg.log_level         = value.parse().unwrap(),
        USE_ID_PREFIX     => cfg.use_id_prefix     = value.parse().unwrap(),
        ID_PREFIX_WIDTH   => cfg.id_prefix_width   = value.parse().unwrap(),
        _                 => {
            error!(get_logger(), "Unknown config property [{}]", name);
            return Ok(());
        }
    }
    store_config(config, name, &cfg);

    Ok(())
}

pub fn get_config_from_name(config: &str) -> AdrToolConfig {
    let cfg: AdrToolConfig = match confy::load(config, None) {
        Err(_why) => AdrToolConfig::default(),
        Ok(e) => e,
    };

    cfg
}

#[cfg(test)]
mod tests {
    use directories::ProjectDirs;
    use std::fs::{self};
    use std::path::Path;
    use uuid::*;
    extern crate slog;
    extern crate slog_term;
    use slog::*;

    #[test]
    fn test_set_config_log_level() {
        let uuid = Uuid::new_v4();
        let name = format!("adrust-tools-4-tests-{}", uuid);
        let config = name.as_str();

        info!(
            super::get_logger(),
            "test_set_config_log_level will use [{}]", config
        );

        super::set_config_from_name(config, "log_level", "7").unwrap();
        let cfg = super::get_config_from_name(config);

        assert_eq!(cfg.log_level, 7);

        teardown(config);
    }

    #[test]
    fn test_set_config_use_id() {
        let uuid = Uuid::new_v4();
        let name = format!("adrust-tools-4-tests-{}", uuid);
        let config = name.as_str();

        info!(
            super::get_logger(),
            "test_set_config_use_id will use [{}]", config
        );

        super::set_config_from_name(config, "use_id_prefix", "false").unwrap();
        let cfg = super::get_config_from_name(config);

        assert_eq!(cfg.use_id_prefix, false);

        teardown(config);
    }

    #[test]
    fn test_set_config_id_width() {
        let uuid = Uuid::new_v4();
        let name = format!("adrust-tools-4-tests-{}", uuid);
        let config = name.as_str();

        info!(
            super::get_logger(),
            "test_set_config_id_width will use [{}]", config
        );

        super::set_config_from_name(config, "id_prefix_width", "10").unwrap();
        let cfg = super::get_config_from_name(config);

        assert_eq!(cfg.id_prefix_width, 10);

        teardown(config);
    }

    fn teardown(name: &str) {
        println!("Want to delete folders [{:?}]", name);
        //delete confy files
        if let Some(dir) = ProjectDirs::from("rs", name, "") {
            if dir.config_dir().exists() {
                let dir = dir.config_dir().to_str().unwrap_or_default();
                match fs::remove_dir_all(dir) {
                    Ok(_val) => {
                        println!("deleted test folders [{:?}]", dir);
                    }
                    Err(_why) => {
                        println!("Problem while deleting test folder");
                    }
                }
            } else {
                println!("Unable to delete folder [{:?}]", dir.config_dir());
            }
        }
    }

    #[test]
    fn test_set_config_adr_root() {
        let uuid = Uuid::new_v4();
        let name = format!("adrust-tools-4-tests-{}", uuid);
        let config = name.as_str();

        info!(
            super::get_logger(),
            "test_set_config_adr_root will use [{}]", config
        );

        match super::set_config_from_name(config, "adr_root_dir", "/tmp/adr-samples-4-tests") {
            Ok(e) => e,
            Err(why) => {
                println!("error in test : {}", why);
            }
        };
        let cfg = super::get_config_from_name(config);
        //assert_eq!(cfg.adr_root_dir, "/tmp/adr-samples-4-tests");
        let index_path = Path::new("/tmp/adr-samples-4-tests/.index");
        assert_eq!(Path::new(cfg.adr_search_index.as_str()), index_path);
        let template_dir_path = Path::new("/tmp/adr-samples-4-tests/templates");
        assert_eq!(Path::new(cfg.adr_template_dir.as_str()), template_dir_path);
        assert_eq!(cfg.adr_template_file, "adr-template-v0.1.adoc");

        teardown(config);
    }

    #[test]
    fn test_set_config_adr_template_file() {
        let uuid = Uuid::new_v4();
        let name = format!("adrust-tools-4-tests-{}", uuid);
        let config = name.as_str();

        info!(
            super::get_logger(),
            "test_set_config_adr_template_file will use [{}]", config
        );

        match super::set_config_from_name(config, "adr_template_file", "new-template.adoc") {
            Ok(e) => e,
            Err(why) => {
                println!("error in test : {}", why);
            }
        }
        let cfg = super::get_config_from_name(config);
        assert_eq!(cfg.adr_template_file, "new-template.adoc");

        teardown(config);
    }

    #[test]
    fn test_set_config_adr_src_dir() {
        let uuid = Uuid::new_v4();
        let name = format!("adrust-tools-4-tests-{}", uuid);
        let config = name.as_str();

        info!(
            super::get_logger(),
            "test_set_config_adr_src_dir will use [{}]", config
        );

        match super::set_config_from_name(config, "adr_src_dir", "/tmp/does-not-exists/src") {
            Ok(e) => e,
            Err(why) => {
                println!("error in test : {}", why);
            }
        }
        let cfg = super::get_config_from_name(config);
        let target_src_dir = Path::new("/tmp/does-not-exists/src");
        assert_eq!(Path::new(cfg.adr_src_dir.as_str()), target_src_dir);
        assert_eq!(cfg.adr_template_file, "adr-template-v0.1.adoc");
        let target_template_dir = Path::new("/tmp/adr-samples/templates");
        assert_eq!(
            Path::new(cfg.adr_template_dir.as_str()),
            target_template_dir
        );

        teardown(config);
    }

    #[test]
    fn test_init() {
        let uuid = Uuid::new_v4();
        let name = format!("adrust-tools-4-tests-{}", uuid);
        let config = name.as_str();

        let project_dirs: ProjectDirs = match ProjectDirs::from("some", config, "") {
            None => panic!("issue while preparing test"),
            Some(project_dirs) => project_dirs,
        };

        info!(super::get_logger(), "test_init will use [{}]", config);

        match super::set_config_from_name(
            config,
            super::ADR_ROOT_DIR,
            format!("{}", project_dirs.cache_dir().display()).as_str(),
        ) {
            Ok(_r) => {
                let _void = super::init_from_name(config);
            }
            Err(why) => {
                panic!("{:?}", why);
            }
        }
        //

        teardown(config);
    }
}
