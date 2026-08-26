use std::fs;
use std::process::Command;

#[test]
fn derive_compiles_with_only_compote_as_a_dependency() {
    let fixture = tempfile::tempdir().expect("create downstream fixture directory");
    let source_dir = fixture.path().join("src");
    fs::create_dir(&source_dir).expect("create downstream source directory");

    let compote_path = env!("CARGO_MANIFEST_DIR").replace('\\', "\\\\");
    fs::write(
        fixture.path().join("Cargo.toml"),
        format!(
            r#"[package]
name = "compote-downstream-derive"
version = "0.0.0"
edition = "2021"

[workspace]

[dependencies]
compote = {{ path = "{compote_path}", features = ["regex", "chrono"] }}
"#,
        ),
    )
    .expect("write downstream manifest");

    fs::write(
        source_dir.join("main.rs"),
        r#"#![deny(unused_imports)]

#[derive(compote::Config)]
struct Minimal {
    name: String,
}

#[derive(compote::Config)]
struct Package {
    name: String,
    version: String,
}

#[derive(compote::Config)]
struct Nested {
    enabled: bool,
}

#[derive(compote::Config)]
struct ExercisesGeneratedDependencyPaths {
    #[compote(flatten)]
    nested: Nested,
    #[compote(allow_map(key = "name", scalar_as = "version"))]
    packages: Vec<Package>,
    #[compote(datetime = "%Y-%m-%d")]
    date: String,
}

#[derive(compote::Config)]
#[compote(external_tag)]
enum RegexVariant {
    #[compote(variant = regex(r"^v\d+$"))]
    Version(String),
    #[compote(variant = any_string)]
    Other(String),
}

fn main() {
    let _ = core::mem::size_of::<Minimal>();
    let _ = core::mem::size_of::<ExercisesGeneratedDependencyPaths>();
    let _ = core::mem::size_of::<RegexVariant>();
}
"#,
    )
    .expect("write downstream source");

    let output = Command::new(env!("CARGO"))
        .arg("check")
        .arg("--quiet")
        .current_dir(fixture.path())
        .env("CARGO_TARGET_DIR", fixture.path().join("target"))
        .output()
        .expect("run cargo check for downstream fixture");

    assert!(
        output.status.success(),
        "downstream crate failed to compile:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
