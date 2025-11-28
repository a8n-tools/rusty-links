//! Server functions for Dioxus fullstack
//!
//! These functions are marked with #[server] and can be called from the client.
//! They run on the server and have access to the database pool and configuration.

pub mod auth;
pub mod links;
