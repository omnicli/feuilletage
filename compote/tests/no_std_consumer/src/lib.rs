#![no_std]

extern crate alloc;

use alloc::{string::String, vec::Vec};

#[derive(compote::Config)]
struct Nested {
    name: String,
}

#[derive(compote::Config)]
struct Package {
    name: String,
    version: String,
}

#[derive(compote::Config)]
struct Root {
    #[compote(flatten)]
    nested: Nested,
    #[compote(allow_map(key = "name", scalar_as = "version"))]
    packages: Vec<Package>,
}

#[derive(compote::Config)]
#[compote(scalar_as = "value")]
struct Wrapped {
    value: String,
}

pub fn assert_derives_compile() {
    let _ = core::mem::size_of::<Root>();
    let _ = core::mem::size_of::<Wrapped>();
    let _: compote::OrderedMap<String, compote::ContextValue> = Default::default();
}
