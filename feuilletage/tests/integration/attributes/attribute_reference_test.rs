use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn parser_attribute_names(source: &str) -> BTreeSet<String> {
    let marker = "meta.path.is_ident(\"";
    let mut names = BTreeSet::new();

    for line in source.lines() {
        let mut remainder = line;
        while let Some(start) = remainder.find(marker) {
            remainder = &remainder[start + marker.len()..];
            let end = remainder.find('"').expect("unterminated is_ident name");
            names.insert(remainder[..end].to_string());
            remainder = &remainder[end + 1..];
        }
    }

    names
}

fn executable_doctest_source(source: &str) -> String {
    let mut code = String::new();
    let mut in_code = false;
    let mut executable = false;

    for line in source.lines() {
        let doc = line
            .strip_prefix("//! ")
            .or_else(|| line.strip_prefix("//!"));
        let Some(doc) = doc else { continue };

        if doc.starts_with("```") {
            if in_code {
                in_code = false;
                executable = false;
            } else {
                in_code = true;
                let language = doc.trim_start_matches('`').trim();
                executable = language.is_empty() || language == "rust";
            }
        } else if executable {
            code.push_str(doc);
            code.push('\n');
        }
    }

    code
}

fn documented_attribute_names(code: &str) -> BTreeSet<String> {
    let marker = "#[feuilletage(";
    let bytes = code.as_bytes();
    let mut names = BTreeSet::new();
    let mut offset = 0;

    while let Some(relative_start) = code[offset..].find(marker) {
        let mut index = offset + relative_start + marker.len();
        let mut paren_depth = 1;
        let mut bracket_depth = 0;
        let mut expect_name = true;

        while index < bytes.len() && paren_depth > 0 {
            match bytes[index] {
                b'(' => paren_depth += 1,
                b')' => paren_depth -= 1,
                b'[' => bracket_depth += 1,
                b']' => bracket_depth -= 1,
                b',' if paren_depth == 1 && bracket_depth == 0 => expect_name = true,
                byte if expect_name
                    && paren_depth == 1
                    && bracket_depth == 0
                    && (byte == b'_' || byte.is_ascii_alphabetic()) =>
                {
                    let start = index;
                    index += 1;
                    while index < bytes.len()
                        && (bytes[index] == b'_' || bytes[index].is_ascii_alphanumeric())
                    {
                        index += 1;
                    }
                    names.insert(code[start..index].to_string());
                    expect_name = false;
                    continue;
                }
                _ => {}
            }
            index += 1;
        }

        offset = index;
    }

    names
}

#[test]
fn every_parser_attribute_has_an_executable_reference_example() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parser = fs::read_to_string(crate_dir.join("../feuilletage-macros/src/attrs.rs"))
        .expect("read macro attribute parser");
    let reference = fs::read_to_string(crate_dir.join("src/attributes.rs"))
        .expect("read executable attribute reference");

    let parser_names = parser_attribute_names(&parser);
    let documented_names = documented_attribute_names(&executable_doctest_source(&reference));
    let missing: Vec<_> = parser_names.difference(&documented_names).collect();

    assert!(
        missing.is_empty(),
        "parser attributes missing from executable feuilletage/src/attributes.rs doctests: {missing:?}"
    );
}
