//! Authentication module for Rusty Links.
//!
//! - `jwt`      — HS256 JWT creation/validation (standalone mode)
//! - `middleware` — Axum extractors for authentication
//! - `oidc_rs`  — OIDC Resource Server token verifier (saas mode)
//! - `oidc_rp`  — OIDC Relying Party BFF handlers (saas mode)

#[cfg(feature = "standalone")]
pub mod jwt;
pub mod middleware;
#[cfg(feature = "saas")]
pub mod oidc_rp;
#[cfg(feature = "saas")]
pub mod oidc_rs;
