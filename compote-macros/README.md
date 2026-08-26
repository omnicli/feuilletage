# compote-macros

Procedural macro implementation for the
[`compote`](https://crates.io/crates/compote) configuration library.

Most users should depend on `compote` rather than this crate directly. The
library re-exports `#[derive(compote::Config)]` and provides the runtime types
used by the generated code.

See the [Compote documentation](https://docs.rs/compote) for installation,
examples, and the complete derive attribute reference.

## License

Licensed under either Apache-2.0 or MIT, at your option.
