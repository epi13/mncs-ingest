//! `mncs-ingest`: machine-native ingestion and semantic normalization.
//!
//! Heterogeneous external representations enter through [`adapters`] and
//! converge into canonical [`ir::Fragment`] values. Closed-vocabulary
//! semantic verdicts execute in MNCS Language ([`language`]); the host
//! carries bytes, layout, and transport — never duplicate policy.

pub mod adapters;
pub mod canonical;
pub mod consumer;
pub mod csv;
pub mod embed;
pub mod manifest;
pub mod error;
pub mod frames;
pub mod ir;
pub mod language;

pub use error::IngestError;
