use feuilletage::Config;

// ============================================================================
// variant_value tests
// ============================================================================

// Test 1: variant_value on external_tag enum
#[derive(Debug, PartialEq, Config)]
#[feuilletage(external_tag)]
enum AuthConfig {
    #[feuilletage(rename = "skip", variant = "skip", variant_value = true)]
    Skip(bool),

    #[feuilletage(rename = "token", variant = predicate("is_all_caps_auth"), variant = any_string)]
    Token(String),

    #[feuilletage(rename = "token_env_var")]
    TokenEnvVar(String),
}

fn is_all_caps_auth<S: feuilletage::CustomSource, L: feuilletage::CustomLevel>(
    value: &feuilletage::ContextValue<S, L>,
) -> bool {
    if let feuilletage::ContextValue::String(s, _) = value {
        !s.is_empty() && s.chars().all(|c| c.is_uppercase() || c == '_')
    } else {
        false
    }
}

#[test]
fn test_variant_value_exact_string() {
    // String "skip" -> Skip(true) via variant_value
    let value = feuilletage::ContextValue::<feuilletage::Source, feuilletage::Level>::string(
        "skip".to_string(),
        Default::default(),
    );
    let mut tracker = feuilletage::ErrorTracker::new();
    let result: AuthConfig =
        feuilletage::FromContextValue::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, AuthConfig::Skip(true));
}

#[test]
fn test_variant_value_map_dispatch() {
    // Object {skip: false} -> Skip(false) via map dispatch (not variant_value)
    let mut obj = indexmap::IndexMap::new();
    obj.insert(
        "skip".to_string(),
        feuilletage::ContextValue::<feuilletage::Source, feuilletage::Level>::bool(
            false,
            Default::default(),
        ),
    );
    let value = feuilletage::ContextValue::object(obj, Default::default());
    let mut tracker = feuilletage::ErrorTracker::new();
    let result: AuthConfig =
        feuilletage::FromContextValue::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, AuthConfig::Skip(false));
}

#[test]
fn test_variant_value_map_dispatch_true() {
    // Object {skip: true} -> Skip(true) via map dispatch
    let mut obj = indexmap::IndexMap::new();
    obj.insert(
        "skip".to_string(),
        feuilletage::ContextValue::<feuilletage::Source, feuilletage::Level>::bool(
            true,
            Default::default(),
        ),
    );
    let value = feuilletage::ContextValue::object(obj, Default::default());
    let mut tracker = feuilletage::ErrorTracker::new();
    let result: AuthConfig =
        feuilletage::FromContextValue::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, AuthConfig::Skip(true));
}

#[test]
fn test_predicate_with_map_dispatch() {
    // ALL_CAPS string -> Token (via predicate)
    let value = feuilletage::ContextValue::<feuilletage::Source, feuilletage::Level>::string(
        "MY_TOKEN".to_string(),
        Default::default(),
    );
    let mut tracker = feuilletage::ErrorTracker::new();
    let result: AuthConfig =
        feuilletage::FromContextValue::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, AuthConfig::Token("MY_TOKEN".to_string()));
}

#[test]
fn test_any_string_with_map_dispatch() {
    // Regular string -> Token (via any_string)
    let value = feuilletage::ContextValue::<feuilletage::Source, feuilletage::Level>::string(
        "my-token-value".to_string(),
        Default::default(),
    );
    let mut tracker = feuilletage::ErrorTracker::new();
    let result: AuthConfig =
        feuilletage::FromContextValue::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, AuthConfig::Token("my-token-value".to_string()));
}

#[test]
fn test_token_env_var_map_dispatch() {
    // Object {token_env_var: "VAR"} -> TokenEnvVar via map dispatch
    let mut obj = indexmap::IndexMap::new();
    obj.insert(
        "token_env_var".to_string(),
        feuilletage::ContextValue::<feuilletage::Source, feuilletage::Level>::string(
            "MY_VAR".to_string(),
            Default::default(),
        ),
    );
    let value = feuilletage::ContextValue::object(obj, Default::default());
    let mut tracker = feuilletage::ErrorTracker::new();
    let result: AuthConfig =
        feuilletage::FromContextValue::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, AuthConfig::TokenEnvVar("MY_VAR".to_string()));
}

#[test]
fn test_token_map_dispatch() {
    // Object {token: "abc"} -> Token via map dispatch (note: Token also has scalar matching)
    let mut obj = indexmap::IndexMap::new();
    obj.insert(
        "token".to_string(),
        feuilletage::ContextValue::<feuilletage::Source, feuilletage::Level>::string(
            "abc".to_string(),
            Default::default(),
        ),
    );
    let value = feuilletage::ContextValue::object(obj, Default::default());
    let mut tracker = feuilletage::ErrorTracker::new();
    let result: AuthConfig =
        feuilletage::FromContextValue::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, AuthConfig::Token("abc".to_string()));
}

// ============================================================================
// variant_default tests
// ============================================================================

#[derive(Debug, PartialEq, Default, Config)]
#[feuilletage(scalar_as = "hostname")]
struct GhCliConfig {
    #[feuilletage(default)]
    hostname: Option<String>,
    #[feuilletage(default)]
    user: Option<String>,
}

#[derive(Debug, PartialEq, Config)]
#[feuilletage(external_tag)]
enum AuthWithGh {
    #[feuilletage(rename = "gh", variant = "gh", variant_default)]
    GhCli(GhCliConfig),

    #[feuilletage(rename = "token", variant = any_string)]
    Token(String),
}

#[test]
fn test_variant_default_string_match() {
    // String "gh" -> GhCli(GhCliConfig::default()) via variant_default
    let value = feuilletage::ContextValue::<feuilletage::Source, feuilletage::Level>::string(
        "gh".to_string(),
        Default::default(),
    );
    let mut tracker = feuilletage::ErrorTracker::new();
    let result: AuthWithGh =
        feuilletage::FromContextValue::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, AuthWithGh::GhCli(GhCliConfig::default()));
}

#[test]
fn test_variant_default_map_dispatch_string() {
    // Object {gh: "myhost"} -> GhCli with hostname via scalar_as on GhCliConfig
    let mut obj = indexmap::IndexMap::new();
    obj.insert(
        "gh".to_string(),
        feuilletage::ContextValue::<feuilletage::Source, feuilletage::Level>::string(
            "myhost".to_string(),
            Default::default(),
        ),
    );
    let value = feuilletage::ContextValue::object(obj, Default::default());
    let mut tracker = feuilletage::ErrorTracker::new();
    let result: AuthWithGh =
        feuilletage::FromContextValue::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(
        result,
        AuthWithGh::GhCli(GhCliConfig {
            hostname: Some("myhost".to_string()),
            user: None
        })
    );
}

#[test]
fn test_variant_default_map_dispatch_object() {
    // Object {gh: {hostname: "h", user: "u"}} -> GhCli with both fields
    let mut gh_obj = indexmap::IndexMap::new();
    gh_obj.insert(
        "hostname".to_string(),
        feuilletage::ContextValue::<feuilletage::Source, feuilletage::Level>::string(
            "h".to_string(),
            Default::default(),
        ),
    );
    gh_obj.insert(
        "user".to_string(),
        feuilletage::ContextValue::<feuilletage::Source, feuilletage::Level>::string(
            "u".to_string(),
            Default::default(),
        ),
    );
    let gh_value = feuilletage::ContextValue::object(gh_obj, Default::default());
    let mut obj = indexmap::IndexMap::new();
    obj.insert("gh".to_string(), gh_value);
    let value = feuilletage::ContextValue::object(obj, Default::default());
    let mut tracker = feuilletage::ErrorTracker::new();
    let result: AuthWithGh =
        feuilletage::FromContextValue::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(
        result,
        AuthWithGh::GhCli(GhCliConfig {
            hostname: Some("h".to_string()),
            user: Some("u".to_string())
        })
    );
}
