use std::path::Path;
use std::process;
use blueprint_tangle_extra::blueprint;
use experimental_blueprint_lib::square;

fn main() {
    let contract_dirs: Vec<&str> = vec!["./contracts"];
    blueprint_build_utils::soldeer_install();
    blueprint_build_utils::soldeer_update();
    blueprint_build_utils::build_contracts(contract_dirs);

    println!("cargo::rerun-if-changed=../experimental-blueprint-lib");

    let blueprint = blueprint! {
        name: "experiment",
        master_manager_revision: "Latest",
        manager: { Evm = "ExperimentalBlueprint" },
        jobs: [square]
    };

    match blueprint {
        Ok(blueprint) => {
            // TODO: Should be a helper function probably
            let json = blueprint_tangle_extra::metadata::macros::ext::serde_json::to_string_pretty(&blueprint).unwrap();
            std::fs::write(Path::new(env!("CARGO_MANIFEST_DIR")).join("blueprint.json"), json.as_bytes()).unwrap();
        },
        Err(e) =>  {
            println!("cargo::error={e:?}");
            process::exit(1);
        }
    }
}
