#![no_std]

extern crate alloc;

use alloc::{string::String, vec::Vec};

#[derive(feuilletage::Config)]
struct Nested {
    name: String,
}

#[derive(feuilletage::Config)]
struct Package {
    name: String,
    version: String,
}

#[derive(feuilletage::Config)]
struct Root {
    #[feuilletage(flatten)]
    nested: Nested,
    #[feuilletage(allow_map(key = "name", scalar_as = "version"))]
    packages: Vec<Package>,
}

#[derive(feuilletage::Config)]
#[feuilletage(scalar_as = "value")]
struct Wrapped {
    value: String,
}

pub fn assert_derives_compile() {
    let _ = core::mem::size_of::<Root>();
    let _ = core::mem::size_of::<Wrapped>();
    let _: feuilletage::OrderedMap<String, feuilletage::ContextValue> = Default::default();
}
