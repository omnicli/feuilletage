//! Unit tests for edit module (path parsing and IntoPath trait).
//!
//! Extracted from feuilletage/src/edit.rs

use feuilletage::edit::IntoPath;

/// Helper function to parse paths (mirrors the private parse_path function)
fn parse_path(path: &str) -> Vec<String> {
    if path.is_empty() {
        return Vec::new();
    }

    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if chars.peek() == Some(&'.') {
                    chars.next();
                    current.push('.');
                } else {
                    current.push('\\');
                }
            }
            '.' => {
                if !current.is_empty() {
                    segments.push(current);
                    current = String::new();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

#[test]
fn test_parse_path_simple() {
    assert_eq!(parse_path("a.b.c"), vec!["a", "b", "c"]);
    assert_eq!(parse_path("a"), vec!["a"]);
    assert_eq!(parse_path(""), Vec::<String>::new());
}

#[test]
fn test_parse_path_escaped_dots() {
    assert_eq!(parse_path("a.b\\.c.d"), vec!["a", "b.c", "d"]);
    assert_eq!(parse_path("a\\.b\\.c"), vec!["a.b.c"]);
    assert_eq!(parse_path("a\\.b.c"), vec!["a.b", "c"]);
}

#[test]
fn test_parse_path_edge_cases() {
    // Trailing backslash (not followed by dot)
    assert_eq!(parse_path("a\\b"), vec!["a\\b"]);
    // Multiple dots
    assert_eq!(parse_path("a..b"), vec!["a", "b"]);
}

#[test]
fn test_into_path_slice() {
    let path: Vec<String> = ["a", "b.c", "d"].as_slice().into_path();
    assert_eq!(path, vec!["a", "b.c", "d"]);
}

#[test]
fn test_into_path_array() {
    let path: Vec<String> = ["a", "b.c", "d"].into_path();
    assert_eq!(path, vec!["a", "b.c", "d"]);
}

#[test]
fn test_into_path_vec() {
    let v = vec!["a".to_string(), "b".to_string()];
    let path: Vec<String> = v.into_path();
    assert_eq!(path, vec!["a", "b"]);
}
