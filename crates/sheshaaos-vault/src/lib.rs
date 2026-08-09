//! `sheshaaos-vault` — Command Vault and Parameter Auto-Resolver for SheshaAOS.

pub mod inspector;
pub mod resolver;
pub mod snippet;

pub use inspector::FlagInspector;
pub use resolver::ParameterResolver;
pub use snippet::CommandSnippet;
