//! Tests for the container-level `parse_as` projection seam.

#![cfg(feature = "json")]

use std::path::{Path, PathBuf};

use feuilletage::{
    loader, Config, Context, ContextValue, CustomLevel, CustomSource, Error, ErrorTracker, Format,
    FromContextValue, FromParsed, Level, MutabilityInfo, OrderedMap, Source, Value,
};

#[derive(Debug, feuilletage::Config, PartialEq)]
struct StructWire {
    name: String,
    count: i64,
}

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(parse_as = "StructWire")]
struct StructProjection {
    label: String,
}

impl<S: CustomSource, L: CustomLevel> FromParsed<StructWire, S, L> for StructProjection {
    fn from_parsed(
        parsed: StructWire,
        _original: &ContextValue<S, L>,
        _tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        Ok(Self {
            label: format!("{}:{}", parsed.name, parsed.count),
        })
    }
}

#[test]
fn parses_a_struct_wire_type_and_serializes_the_target_shape() {
    let mut config = Config::default();
    config.load_json(
        r#"{"name":"demo","count":3}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    let projected: StructProjection = config.deserialize().unwrap();
    assert_eq!(
        projected,
        StructProjection {
            label: "demo:3".to_string()
        }
    );
    assert_eq!(
        feuilletage::to_json_compact(&projected).unwrap(),
        r#"{"label":"demo:3"}"#
    );
}

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(tag = "kind")]
enum EnumWire {
    Text { value: String },
    Number { value: i64 },
}

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(parse_as = "EnumWire")]
struct EnumProjection {
    rendered: String,
}

impl<S: CustomSource, L: CustomLevel> FromParsed<EnumWire, S, L> for EnumProjection {
    fn from_parsed(
        parsed: EnumWire,
        _original: &ContextValue<S, L>,
        _tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        let rendered = match parsed {
            EnumWire::Text { value } => value,
            EnumWire::Number { value } => value.to_string(),
        };
        Ok(Self { rendered })
    }
}

#[test]
fn parses_an_enum_wire_type() {
    let value: ContextValue = ContextValue::object(
        OrderedMap::from([
            (
                "kind".to_string(),
                ContextValue::string("number", Context::default()),
            ),
            (
                "value".to_string(),
                ContextValue::int(42, Context::default()),
            ),
        ]),
        Context::default(),
    );

    let projected = EnumProjection::from_context_value(&value, &mut ErrorTracker::new()).unwrap();
    assert_eq!(projected.rendered, "42");
}

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(tag = "kind", parse_as = "StructWire")]
enum EnumTargetProjection {
    Summary { label: String },
}

impl<S: CustomSource, L: CustomLevel> FromParsed<StructWire, S, L> for EnumTargetProjection {
    fn from_parsed(
        parsed: StructWire,
        _original: &ContextValue<S, L>,
        _tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        Ok(Self::Summary {
            label: format!("{}:{}", parsed.name, parsed.count),
        })
    }
}

#[test]
fn enum_target_serialization_is_independent_of_projection_wire_format() {
    let mut config = Config::default();
    config.load_json(
        r#"{"name":"demo","count":3}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    let projected: EnumTargetProjection = config.deserialize().unwrap();
    assert_eq!(
        feuilletage::to_json_compact(&projected).unwrap(),
        r#"{"kind":"summary","label":"demo:3"}"#
    );
}

#[derive(Debug, feuilletage::Config)]
struct MutableWire {
    #[feuilletage(mutable_by = ["user"], default = "wire-default")]
    wire_name: String,
}

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(parse_as = "MutableWire")]
struct MutableProjection {
    projected_name: String,
}

impl<S: CustomSource, L: CustomLevel> FromParsed<MutableWire, S, L> for MutableProjection {
    fn from_parsed(
        parsed: MutableWire,
        _original: &ContextValue<S, L>,
        _tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        Ok(Self {
            projected_name: parsed.wire_name,
        })
    }
}

#[test]
fn projection_mutability_uses_wire_field_names_and_constraints() {
    let mut loader = loader()
        .load_str(r#"{"wire_name":"blocked"}"#, Format::Json, Level::System)
        .unwrap();

    let projected: MutableProjection = loader.deserialize().unwrap();

    assert_eq!(projected.projected_name, "wire-default");
    assert_eq!(loader.errors().warnings().len(), 1);
    assert_eq!(loader.errors().warnings()[0].path, "wire_name");
    assert_eq!(
        loader.errors().warnings()[0].message,
        "value from 'system' level ignored (allowed by: [user])"
    );
}

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(tag = "kind", parse_as = "MutableWire")]
enum MutableEnumProjection {
    Value { projected_name: String },
}

impl<S: CustomSource, L: CustomLevel> FromParsed<MutableWire, S, L> for MutableEnumProjection {
    fn from_parsed(
        parsed: MutableWire,
        _original: &ContextValue<S, L>,
        _tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        Ok(Self::Value {
            projected_name: parsed.wire_name,
        })
    }
}

#[test]
fn enum_projection_mutability_uses_wire_constraints() {
    let mut loader = loader()
        .load_str(r#"{"wire_name":"blocked"}"#, Format::Json, Level::System)
        .unwrap();

    let projected: MutableEnumProjection = loader.deserialize().unwrap();

    assert_eq!(
        projected,
        MutableEnumProjection::Value {
            projected_name: "wire-default".to_string()
        }
    );
    assert_eq!(loader.errors().warnings().len(), 1);
    assert_eq!(loader.errors().warnings()[0].path, "wire_name");
}

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(parse_as = "MutableWire", skip_serialize, skip_deserialize)]
struct GenericProjection<T> {
    value: T,
}

impl<S: CustomSource, L: CustomLevel, T: From<String>> FromParsed<MutableWire, S, L>
    for GenericProjection<T>
{
    fn from_parsed(
        parsed: MutableWire,
        _original: &ContextValue<S, L>,
        _tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        Ok(Self {
            value: parsed.wire_name.into(),
        })
    }
}

#[test]
fn projection_mutability_preserves_generic_targets() {
    let mut loader = loader()
        .load_str(r#"{"wire_name":"allowed"}"#, Format::Json, Level::User)
        .unwrap();

    let projected: GenericProjection<String> = loader.deserialize().unwrap();
    assert_eq!(projected.value, "allowed");
    assert!(loader.errors().warnings().is_empty());
}

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(parse_as = "feuilletage::Value")]
struct ValueProjection {
    value: Value,
}

impl<S: CustomSource, L: CustomLevel> FromParsed<Value, S, L> for ValueProjection {
    fn from_parsed(
        parsed: Value,
        _original: &ContextValue<S, L>,
        _tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        Ok(Self { value: parsed })
    }
}

#[test]
fn value_projection_has_no_mutability_constraints() {
    let value: ContextValue = ContextValue::string("dynamic", Context::default());
    let projected = ValueProjection::from_context_value(&value, &mut ErrorTracker::new()).unwrap();

    assert_eq!(projected.value, Value::String("dynamic".to_string()));
    assert!(ValueProjection::mutability_constraints().is_empty());
}

#[derive(Debug, feuilletage::Config)]
#[feuilletage(scalar_as = "value")]
struct ScalarWire {
    value: String,
}

#[derive(Debug, feuilletage::Config)]
#[feuilletage(parse_as = "ScalarWire")]
struct ScalarOriginal {
    parsed: String,
    original: String,
}

impl<S: CustomSource, L: CustomLevel> FromParsed<ScalarWire, S, L> for ScalarOriginal {
    fn from_parsed(
        parsed: ScalarWire,
        original: &ContextValue<S, L>,
        _tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        Ok(Self {
            parsed: parsed.value,
            original: original.as_str().unwrap().to_string(),
        })
    }
}

#[test]
fn preserves_original_value_when_wire_uses_scalar_as() {
    let value: ContextValue = ContextValue::string("raw", Context::default());
    let projected = ScalarOriginal::from_context_value(&value, &mut ErrorTracker::new()).unwrap();

    assert_eq!(projected.parsed, "raw");
    assert_eq!(projected.original, "raw");
}

fn trim_wire<S: CustomSource, L: CustomLevel>(
    value: &mut ContextValue<S, L>,
    context: &Context<S, L>,
) -> Result<(), Error> {
    if let ContextValue::String(text, _) = value {
        *value = ContextValue::string(text.trim(), context.clone());
    }
    Ok(())
}

#[derive(Debug, feuilletage::Config)]
#[feuilletage(transparent, transform = "self::trim_wire")]
struct TransformedWire(String);

#[derive(Debug, feuilletage::Config)]
#[feuilletage(parse_as = "TransformedWire")]
struct TransformOriginal {
    parsed: String,
    original: String,
}

impl<S: CustomSource, L: CustomLevel> FromParsed<TransformedWire, S, L> for TransformOriginal {
    fn from_parsed(
        parsed: TransformedWire,
        original: &ContextValue<S, L>,
        _tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        Ok(Self {
            parsed: parsed.0,
            original: original.as_str().unwrap().to_string(),
        })
    }
}

#[test]
fn preserves_original_value_when_wire_uses_transform() {
    let value: ContextValue = ContextValue::string("  raw  ", Context::default());
    let projected =
        TransformOriginal::from_context_value(&value, &mut ErrorTracker::new()).unwrap();

    assert_eq!(projected.parsed, "raw");
    assert_eq!(projected.original, "  raw  ");
}

#[derive(Debug, feuilletage::Config)]
struct ErrorWire {
    value: i64,
}

#[derive(Debug, feuilletage::Config)]
#[feuilletage(parse_as = "ErrorWire")]
struct ProjectionError {
    #[allow(dead_code)]
    value: i64,
}

impl<S: CustomSource, L: CustomLevel> FromParsed<ErrorWire, S, L> for ProjectionError {
    fn from_parsed(
        _parsed: ErrorWire,
        _original: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        Err(Error::InvalidValue {
            path: tracker.current_path(),
            message: "projection failed".to_string(),
        })
    }
}

#[derive(Debug, feuilletage::Config)]
struct ProjectionErrorContainer {
    projected: ProjectionError,
}

#[test]
fn preserves_tracker_path_on_projection_error() {
    let mut config = Config::default();
    config.load_json(
        r#"{"projected":{"value":1}}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    let error = config
        .deserialize::<ProjectionErrorContainer>()
        .unwrap_err();
    assert!(matches!(
        error,
        Error::InvalidValue { path, message }
            if path == "projected" && message == "projection failed"
    ));
}

#[derive(Clone, Debug, Default, PartialEq)]
enum ProjectionSource {
    #[default]
    Programmatic,
    File(PathBuf),
}

impl CustomSource for ProjectionSource {
    fn display_name(&self) -> String {
        format!("{self:?}")
    }

    fn file_path(&self) -> Option<&Path> {
        match self {
            Self::File(path) => Some(path),
            Self::Programmatic => None,
        }
    }

    fn from_file(path: PathBuf) -> Self {
        Self::File(path)
    }

    fn programmatic() -> Self {
        Self::Programmatic
    }

    fn environment() -> Self {
        Self::Programmatic
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
enum ProjectionLevel {
    #[default]
    Base,
    Project,
}

impl CustomLevel for ProjectionLevel {
    fn name(&self) -> &str {
        match self {
            Self::Base => "base",
            Self::Project => "project",
        }
    }
}

#[derive(Debug, feuilletage::Config)]
struct CustomWire {
    value: String,
}

#[derive(Debug, feuilletage::Config)]
#[feuilletage(parse_as = "CustomWire")]
struct CustomProjection {
    value: String,
    source: String,
    level: String,
}

impl<S: CustomSource, L: CustomLevel> FromParsed<CustomWire, S, L> for CustomProjection {
    fn from_parsed(
        parsed: CustomWire,
        original: &ContextValue<S, L>,
        _tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        Ok(Self {
            value: parsed.value,
            source: original.context().source.display_name(),
            level: original.context().level.name().to_string(),
        })
    }
}

#[test]
fn supports_custom_source_and_level_types() {
    let context = Context::new(
        ProjectionSource::File(PathBuf::from("config.test")),
        ProjectionLevel::Project,
    );
    let value: ContextValue<ProjectionSource, ProjectionLevel> = ContextValue::object(
        OrderedMap::from([(
            "value".to_string(),
            ContextValue::string("custom", context.clone()),
        )]),
        context,
    );

    let projected = <CustomProjection as FromContextValue<
        ProjectionSource,
        ProjectionLevel,
    >>::from_context_value(&value, &mut ErrorTracker::new())
    .unwrap();

    assert_eq!(projected.value, "custom");
    assert_eq!(projected.source, "File(\"config.test\")");
    assert_eq!(projected.level, "project");
}

#[derive(Debug, feuilletage::Config)]
#[feuilletage(parse_as = "StructWire", skip_serialize, skip_deserialize)]
struct ParseOnlyProjection {
    label: String,
}

impl<S: CustomSource, L: CustomLevel> FromParsed<StructWire, S, L> for ParseOnlyProjection {
    fn from_parsed(
        parsed: StructWire,
        _original: &ContextValue<S, L>,
        _tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        Ok(Self { label: parsed.name })
    }
}

#[test]
fn skip_serialize_and_skip_deserialize_leave_projection_available() {
    let value: ContextValue = ContextValue::object(
        OrderedMap::from([
            (
                "name".to_string(),
                ContextValue::string("parse-only", Context::default()),
            ),
            (
                "count".to_string(),
                ContextValue::int(1, Context::default()),
            ),
        ]),
        Context::default(),
    );

    let projected =
        ParseOnlyProjection::from_context_value(&value, &mut ErrorTracker::new()).unwrap();
    assert_eq!(projected.label, "parse-only");
}
