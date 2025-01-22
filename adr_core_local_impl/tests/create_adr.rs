use cucumber::{given, then, when, World};

extern crate directories;
use directories::ProjectDirs;

use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use chrono::Utc;
use walkdir::WalkDir;
use adr_core::adr_repo::{Status, TransitionStatus};

#[derive(Debug, World)]
pub struct AdrNames {
    name: String,
    has_transitioned: bool,
    base_path: String,
    scenario_name: String
}

impl std::default::Default for AdrNames {
    fn default() -> AdrNames {
        AdrNames {
            name: "default-name".to_string(),
            has_transitioned: false,
            base_path: String::from(""),
            scenario_name: "default-name".to_string(),
        }
    }
}

mod helper {
    use std::io::{self};
    extern crate directories;
    use directories::ProjectDirs;

    pub fn create_decision(name: &str, scenario_name: &String) -> io::Result<bool> {
        let project_dirs: ProjectDirs = match ProjectDirs::from("murex", "adrust-tool", self::get_workspace(scenario_name).as_str()) {
            None => panic!("issue while preparing test"),
            Some(project_dirs) => project_dirs,
        };

        let mut cfg: adr_config::config::AdrToolConfig = adr_config::config::get_config();
        cfg.use_id_prefix = false;
        cfg.log_level = 6;
        cfg.adr_template_file = String::from(
            project_dirs
                .cache_dir()
                .join("templates/adr-template-v0.1.adoc")
                .as_path()
                .to_str()
                .unwrap(),
        );
        cfg.adr_src_dir = String::from(
            project_dirs
                .cache_dir()
                .join("src")
                .as_path()
                .to_str()
                .unwrap(),
        );

        Ok(adr_core::adr_repo::create_adr(cfg, None, name).unwrap())
    }

    pub fn get_workspace(scenario_name: &String) -> String {
        let val = format!("test-{}", scenario_name.replace(" ","-"));

        return val.clone();
    }
}


#[given(regex=r"^A decision (.+) I need to make$")]
fn for_this_decision(world: &mut AdrNames, decision_name: String) {
    world.name = decision_name;
}

#[when("I create a new Decision Record")]
fn list_all_tags(world: &mut AdrNames) {
    //do nothing
}


#[then(regex=r"A new file named (.+) is created$")]
fn check_tags(adr: &mut AdrNames, name: String) {
    let is_created = helper::create_decision(&adr.name, &adr.scenario_name).unwrap();
    assert_eq!(is_created, true);

    // TODO there is certainly a way to return project_dir as part of create_decision
    let project_dirs: ProjectDirs = match ProjectDirs::from("murex", "adrust-tool", helper::get_workspace(&adr.scenario_name).as_str()) {
        None => panic!("issue while preparing test"),
        Some(project_dirs) => project_dirs
    };
    let t = project_dirs.cache_dir().join(
        format!("src/{}.adoc", name.to_string())
    );
    //
    let expected_path = t.as_path();
    assert_eq!(expected_path.exists(), true);
    //
    let content: String = fs::read_to_string(expected_path).unwrap();
    assert!(content.contains("{wip}"));
}


#[given(regex=r"^A new decision (.+) that already exists$")]
fn new_decision_exists(adr: &mut AdrNames, name: String) {
    adr.name = name.to_string();
    //the decision should already exist so we create it.
    helper::create_decision(&adr.name, &adr.scenario_name).unwrap();
}

#[when("I create a new ADR")]
fn create_new_adr(adr: &mut AdrNames) {
    //do nothing
}

#[then("The creation fails")]
fn new_adr_creation_fails(adr: &mut AdrNames) {
    //create the same file
    let is_created = helper::create_decision(&adr.name, &adr.scenario_name).unwrap();
    assert_eq!(is_created, false);
}



#[given(regex=r"^a decision with status (.+)$")]
fn a_decision_with_status(adr: &mut AdrNames, status: String) {

    let project_dirs: ProjectDirs = match ProjectDirs::from("murex", "adrust-tool", helper::get_workspace(&adr.scenario_name).as_str()) {
        None => panic!("issue while preparing test"),
        Some(project_dirs) => project_dirs
    };

    //copy all files to ease the different transitions
    let rep = "./tests/data/";
    let srcdir = PathBuf::from(rep);

    for entry in WalkDir::new(srcdir.as_path()).into_iter().filter_map(|e| e.ok() ) {
        if ! entry.path().is_dir() {
            let from = entry.path();
            let file_name = entry.path().file_name().unwrap();
            let to = project_dirs.cache_dir().join("src").join(file_name);

            println!("[init] Want to copy file [{:?}] to [{:?}]", from, to);
            match fs::copy(from, to.as_path()) {
                Ok(_) => (),
                Err(why) => panic!("{:?}", why),
            };
        }
    }

    adr.base_path = format!("{}", project_dirs.cache_dir().join("src").display());
    adr.name = format!("{}", project_dirs.cache_dir().join("src").join(status).with_extension("adoc").display());
    println!("The current scenario will use adr path [{}]", adr.name);
}

#[when(regex=r"^the decision is transitioned to (.+) by (.+)$")]
fn decision_is_transitioned(adr: &mut AdrNames, transition: String, by: String) {
    match "n/a" == by {
        true => {
            println!("calling transition_to() with [{}] [{}] [{}]", transition, adr.name, by);
            match adr_core::adr_repo::transition_to(TransitionStatus::from_str(transition), Path::new(&adr.base_path), adr.name.as_str(), "") {
                Ok(transitioned) => adr.has_transitioned = transitioned,
                Err(why) => panic!("{:?}", why)
            };
        },
        false => {
            let by = format!("{}", PathBuf::from(adr.base_path.as_str()).join(by).display());
            println!("calling transition_to() with [{}] [{}] [{}]", transition, adr.name, by);
            match adr_core::adr_repo::transition_to(TransitionStatus::from_str(transition), Path::new(&adr.base_path), adr.name.as_str(), by.as_str()) {
                Ok(transitioned) => adr.has_transitioned = transitioned,
                Err(why) => panic!("{:?}", why)
            };
        },
    };
}

#[then(regex=r"^the transition is (.+)$")]
fn the_transition_is(adr: &mut AdrNames, has_transition: bool) {
    //compare expected with actual
    assert_eq!(has_transition, adr.has_transitioned)
}

#[then(regex=r"^the new status is (.+) by (.+)$")]
fn the_new_status_is(adr: &mut AdrNames, new_status: String, by: String) {
    let new_adr = match adr_core::adr_repo::build_adr(Path::new(&adr.base_path), Path::new(adr.name.as_str())) {
        Ok(adr) => adr,
        Err(why) => panic!("{} - {} - {:?}", &adr.base_path, &adr.name.as_str(), why),
    };

    assert_eq!(Status::from_str(new_status), new_adr.status);
    if "n/a" != by && adr.has_transitioned {
        let by = format!("}} {}", by);
        assert_eq!(true, new_adr.content.contains(&by));
    }
}

#[then(regex=r"^the date is updated to today if (.+) is true$")]
fn the_date_is_updated_to_today_if_is_true(adr: &mut AdrNames, is_accepted: bool) {
    let adr = match adr_core::adr_repo::build_adr(Path::new(&adr.base_path), Path::new(adr.name.as_str())) {
        Ok(adr) => adr,
        Err(why) => panic!("{:?}", why),
    };

    if is_accepted {
        assert_eq!(adr.date, Utc::now().date_naive().format("%Y-%m-%d").to_string());
    }
    else {
        assert_eq!(adr.date, "2019-10-28");
    }
}

fn main() {
    futures::executor::block_on(
        AdrNames::cucumber()
            .before(|_feature, _rule, scenario, world| {
                Box::pin(async move {
                    world.scenario_name = format!("{}-{}", scenario.name.clone(), scenario.position.line.clone());

                    let project_dirs: ProjectDirs = match ProjectDirs::from("murex", "adrust-tool", helper::get_workspace(&world.scenario_name).as_str()) {
                        None => panic!("issue while preparing test"),
                        Some(project_dirs) => project_dirs
                    };
                    fs::create_dir_all(project_dirs.cache_dir().join("src").as_path()).unwrap();
                    fs::create_dir_all(project_dirs.cache_dir().join("templates").as_path()).unwrap();

                    fs::copy("./tests/adr-template-v0.1.adoc", project_dirs.cache_dir().join("templates/adr-template-v0.1.adoc").as_path()).unwrap();
                })
            })
            .after(|_feature, _rule, scenario, _ev, world| {
                Box::pin(async move {
                    let project_dirs: ProjectDirs = match ProjectDirs::from("murex", "adrust-tool", helper::get_workspace(&scenario.name).as_str()) {
                        None => panic!("issue while preparing test"),
                        Some(project_dirs) => project_dirs
                    };
                    if project_dirs.cache_dir().exists() {
                        fs::remove_dir_all(project_dirs.cache_dir()).unwrap();
                    }
                    else{
                        println!("{} doest not exist", project_dirs.cache_dir().display());
                    }
                })
            })
            .run_and_exit("features/adr_lifecycle/adr_lifecycle_management.feature")
    );
}