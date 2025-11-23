//! Rusty Links Library
//!
//! This library provides the core functionality for the Rusty Links application.
//! It's primarily used for integration testing.

#[cfg(feature = "server")]
pub mod api;
#[cfg(feature = "server")]
pub mod auth;
#[cfg(feature = "server")]
pub mod config;
#[cfg(feature = "server")]
pub mod error;
#[cfg(feature = "server")]
pub mod github;
#[cfg(feature = "server")]
pub mod models;
#[cfg(feature = "server")]
pub mod scheduler;
#[cfg(feature = "server")]
pub mod scraper;
pub mod ui;
