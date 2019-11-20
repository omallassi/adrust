#[derive(Serialize, Deserialize)]
pub struct AdrToolConfig {
    pub adr_root_dir: String,
    pub adr_src_dir: String,
    pub adr_template_dir: String,
    pub log_level: usize,
}

impl ::std::default::Default for AdrToolConfig {
    fn default() -> AdrToolConfig {        
        AdrToolConfig { 
            adr_root_dir: "/tmp/adr-samples".to_string(), 
            adr_src_dir: "/tmp/adr-samples/src".to_string(),
            adr_template_dir: "/tmp/adr-samples/templates".to_string(),
            log_level: 4, //info
        }
    }
}

pub fn get_config() -> AdrToolConfig {
    let cfg: AdrToolConfig = match confy::load("adrust-tools") {
        Err(_why) => panic!("Unable to access Config Files"),
        Ok(e) => e,
    };

    cfg
}

pub fn store(cfg: AdrToolConfig) {
    confy::store("adrust-tools", cfg).unwrap();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}