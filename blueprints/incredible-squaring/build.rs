fn main() {
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");

    let blueprint = serde_json::json!({
        "metadata": {
            "name": env!("CARGO_PKG_NAME"),
            "description": env!("CARGO_PKG_DESCRIPTION"),
            "author": env!("CARGO_PKG_AUTHORS"),
            "category": "Cryptographic Services",
            "code_repository": env!("CARGO_PKG_REPOSITORY"),
            "logo": "https://example.com/logo.png",
            "website": env!("CARGO_PKG_HOMEPAGE"),
            "license": env!("CARGO_PKG_LICENSE")
        },
        "jobs": [
            {
                "metadata": {
                    "name": "xsquare",
                    "description": "Returns the square of the input as `U256` bytes in BE format."
                },
                "params": ["Bytes"],
                "result": ["Bytes"],
                "verifier": "None"
            }
        ],
        "registration_hook": "None",
        "registration_params": [],
        "request_hook": "None",
        "request_params": [],
        "gadget": {
            "Native": {
                "soruces": [
                    {
                        "fetcher": {
                            "Github": {
                                "owner": "webb-tools",
                                "repo": "gadget",
                                "tag": env!("CARGO_PKG_VERSION"),
                                "binaries": [
                                    { "arch": "Amd64", "os": "Linux", "name": "incredible-squaring-gadget", "sha256": "" },
                                    { "arch": "Arm64", "os": "MacOS", "name": "incredible-squaring-gadget", "sha256": "" },
                                ]
                            }
                        },
                    }
                ]
            }
        }
    });

    // write the blueprint to blueprint.json next to build.rs
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let blueprint_path = current_dir.join("blueprint.json");
    std::fs::write(
        blueprint_path,
        serde_json::to_string_pretty(&blueprint).unwrap(),
    )
    .expect("Failed to write blueprint.json");
}
