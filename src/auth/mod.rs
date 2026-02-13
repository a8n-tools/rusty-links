//! Authentication module for Rusty Links
//!
//! This module provides authentication functionality using JWT tokens.
//!
//! # Modules
//!
//! - `jwt` - JWT token creation and validation (standalone mode)
//! - `middleware` - Axum extractors for authentication
//! - `saas_auth` - SaaS mode authentication via parent app cookies

#[cfg(feature = "standalone")]
pub mod jwt;
pub mod middleware;
#[cfg(feature = "saas")]
pub mod saas_auth;
