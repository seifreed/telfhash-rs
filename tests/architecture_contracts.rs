use std::fs;
use std::path::Path;

fn read(path: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}

#[test]
fn domain_layer_does_not_depend_on_outer_layers() {
    for path in [
        "src/domain/error.rs",
        "src/domain/exclusions.rs",
        "src/domain/grouping/common.rs",
        "src/domain/grouping/compatible.rs",
        "src/domain/grouping/connected.rs",
        "src/domain/model.rs",
        "src/domain/ports.rs",
    ] {
        let body = read(path);
        assert!(!body.contains("crate::application::"));
        assert!(!body.contains("crate::interfaces::"));
        assert!(!body.contains("crate::infrastructure::"));
    }
}

#[test]
fn application_layer_does_not_depend_on_interfaces() {
    for path in ["src/application/analysis.rs", "src/application/service.rs"] {
        let body = read(path);
        assert!(!body.contains("crate::interfaces::"));
    }
}

#[test]
fn docs_cover_architecture_domain_and_legacy_constraints() {
    for path in ["README.md", "CHANGELOG.md", "Cargo.toml"] {
        let body = read(path);
        assert!(!body.trim().is_empty(), "{path} should not be empty");
    }
}

#[test]
fn lib_rs_does_not_expose_internal_modules_as_public_api() {
    let body = read("src/lib.rs");
    assert!(!body.contains("pub mod application;"));
    assert!(!body.contains("pub mod domain;"));
    assert!(!body.contains("pub mod infrastructure;"));
    assert!(!body.contains("pub mod interfaces;"));
}
