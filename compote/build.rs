use std::{env, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let packaged_readme = manifest_dir.join("README.md");
    let readme = if packaged_readme.exists() {
        packaged_readme
    } else {
        manifest_dir.join("../README.md")
    };

    println!("cargo:rerun-if-changed={}", readme.display());
    println!("cargo:rustc-env=COMPOTE_README={}", readme.display());
}
