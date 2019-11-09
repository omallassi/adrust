use cucumber::{cucumber, before, after};

extern crate directories;
use directories::ProjectDirs;

use std::fs;

pub struct Adr {
    name: String,
}

impl cucumber::World for Adr {}
impl std::default::Default for Adr {
    fn default() -> Adr {
        Adr {
            name: "default-name".to_string(),
        }
    }
}

mod helper {
    use std::io::{self};
    extern crate directories;
    use directories::ProjectDirs;

    pub fn create_decision(name: &str) -> io::Result<(bool)> {
        let project_dirs: ProjectDirs = match ProjectDirs::from("murex", "adrust-tool", "test") {
            None => panic!("issue while preparing test"),
            Some(project_dirs) => project_dirs
        };

        Ok(adr_core::adr_repo::create_adr(name, 
            project_dirs.cache_dir().join("templates").as_path(), 
            project_dirs.cache_dir().join("src").as_path()).unwrap())
    }
}

mod create_new_adr_steps {
    use cucumber::steps;
    extern crate directories;
    use directories::ProjectDirs;

    extern crate adr_core;
    use crate::helper;

    steps! (crate::Adr => {
        given regex r"^A decision (.+) I need to make$" (String) |adr, name, _step| {
            adr.name = name.to_string();
        };

        when "I want to create a new Decision Record" |_adr, _step| {

        };

        then "I can create a new ADR" |adr, _step| {
            let is_created = helper::create_decision(&adr.name).unwrap();
            assert_eq!(is_created, true);
            
            // TODO there is certainly a way to return project_dir as part of create_decision
            let project_dirs: ProjectDirs = match ProjectDirs::from("murex", "adrust-tool", "test") {
                None => panic!("issue while preparing test"),
                Some(project_dirs) => project_dirs
            };
            let t = project_dirs.cache_dir().join(format!("src/{}.adoc", &adr.name));
            let expected_path = t.as_path();

            assert_eq!(expected_path.exists(), true);
        };
    });
}

mod create_already_adr_steps {
    use cucumber::steps;
    extern crate adr_core;
    use crate::helper;

    steps! (crate::Adr => {
        given regex r"^A new decision (.+) that already exists$" (String) |adr, name, _step| {
            adr.name = name.to_string();
            //the decision should already exist so we create it. 
            helper::create_decision(&adr.name).unwrap();
        };

        when "I create a new ADR" |_adr, _step| {
            
        };

        then "The creation fails" | adr, _step | {
            //create the same file 
            let is_created = helper::create_decision(&adr.name).unwrap();
            assert_eq!(is_created, false);
        };
    });
}


before!(a_before_fn => |_scenario| {
    let project_dirs: ProjectDirs = match ProjectDirs::from("murex", "adrust-tool", "test") {
        None => panic!("issue while preparing test"),
        Some(project_dirs) => project_dirs
    };
    fs::create_dir_all(project_dirs.cache_dir().join("src").as_path()).unwrap();
    fs::create_dir_all(project_dirs.cache_dir().join("templates").as_path()).unwrap();

    fs::copy("./tests/adr-template-v0.1.adoc", project_dirs.cache_dir().join("templates/adr-template-v0.1.adoc").as_path()).unwrap();
});

after!(an_after_fn => |_scenario| {
    let project_dirs: ProjectDirs = match ProjectDirs::from("murex", "adrust-tool", "test") {
        None => panic!("issue while preparing test"),
        Some(project_dirs) => project_dirs
    };
    fs::remove_dir_all(project_dirs.cache_dir()).unwrap();
});

fn setup() {}


cucumber!{
    features: "./features/create_adr", 
    world: crate::Adr,
    steps: &[
        create_new_adr_steps::steps, 
        create_already_adr_steps::steps,
    ], 
    setup: setup,
    before: &[a_before_fn],
    after: &[an_after_fn]
}