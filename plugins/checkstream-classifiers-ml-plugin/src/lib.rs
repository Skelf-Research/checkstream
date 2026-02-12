//! External ML inference plugin for `checkstream-classifiers`.
//!
//! This crate provides a Candle/HuggingFace-backed implementation of
//! `ModelLoaderPlugin` and can be injected into
//! `checkstream_classifiers::dynamic_registry::DynamicRegistryBuilder`.

pub mod external_ml_loader;

pub use external_ml_loader::ExternalMlModelLoader;
