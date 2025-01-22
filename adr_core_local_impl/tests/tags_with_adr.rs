use cucumber::{World};

extern crate directories;

#[derive(Debug, World)]
pub struct AdrNames {
    name: String,
}

impl std::default::Default for AdrNames {
    fn default() -> AdrNames {
        AdrNames {
            name: "default-name".to_string(),
        }
    }
}
mod check_tags_management {
    use cucumber::{given, then, when};
    extern crate adr_core;
    use std::path::Path;
    use crate::AdrNames;

    #[given(regex = r"^the decision (.+)$")]
    fn for_this_decision(world: &mut AdrNames, decision_name: String) {
        world.name = decision_name;
    }

    #[when("I list all the tags")]
    fn list_all_tags(_world: &mut AdrNames) {
        //nothign to do
    }

    #[then(regex = r"^I got (.+), (.+), (.+) tags$")]
    fn check_tags(adr: &mut AdrNames, tag_1: String, tag_2: String, tag_3: String) {
        let file_path = Path::new(adr.name.as_str());
        let decision = adr_core::adr_repo::build_adr(Path::new(""), file_path).unwrap();

        if decision.tags_array.len() == 1 {
            assert_eq!(decision.tags_array[0].trim(), tag_1);
        }
        if decision.tags_array.len() == 2 {
            assert_eq!(decision.tags_array[1].trim(), tag_2);
        }
        if decision.tags_array.len() == 3 {
            assert_eq!(decision.tags_array[2].trim(), tag_3);
        }
    }
}

fn main() {
    // You may choose any executor you like (`tokio`, `async-std`, etc.).
    // You may even have an `async` main, it doesn't matter. The point is that
    // Cucumber is composable. :)
    futures::executor::block_on(AdrNames::run(
        "features/adr_lifecycle/adr_tags_management.feature",
    ));
}