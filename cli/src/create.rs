pub fn new_blueprint(name: &str) {
    let path = cargo_generate::generate(cargo_generate::GenerateArgs {
        template_path: cargo_generate::TemplatePath {
            auto_path: None,
            subfolder: None,
            test: false,
            git: Some(String::from(
                "https://github.com/tangle-network/blueprint-template/",
            )),
            branch: Some(String::from("main")),
            tag: None,
            revision: None,
            path: None,
            favorite: None,
        },
        list_favorites: false,
        name: Some(name.to_string()),
        force: false,
        verbose: false,
        template_values_file: None,
        silent: false,
        config: None,
        vcs: Some(cargo_generate::Vcs::Git),
        lib: false,
        bin: true,
        ssh_identity: None,
        define: Default::default(),
        init: false,
        destination: None,
        force_git_init: false,
        allow_commands: false,
        overwrite: false,
        skip_submodules: false,
        other_args: Default::default(),
    });

    match path {
        Ok(path) => {
            println!("Blueprint generated at: {}", path.display());
        }
        Err(e) => {
            eprintln!("Error generating blueprint: {}", e);
        }
    }
}
