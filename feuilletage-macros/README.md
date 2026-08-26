# feuilletage-macros

Procedural macro implementation for the
[`feuilletage`](https://crates.io/crates/feuilletage) configuration library.

Most users should depend on `feuilletage` rather than this crate directly. The
library re-exports `#[derive(feuilletage::Config)]` and provides the runtime types
used by the generated code.

See the [Feuilletage documentation](https://docs.rs/feuilletage) for installation,
examples, and the complete derive attribute reference.

## License

Licensed under either Apache-2.0 or MIT, at your option.
