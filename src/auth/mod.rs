//! Authentication module for Rusty Links.
//!
//! - `jwt`      — HS256 JWT creation/validation (standalone mode)
//! - `middleware` — Axum extractors for authentication
//! - `oidc_rs`  — OIDC Resource Server token verifier (hosted mode)
//! - `oidc_rp`  — OIDC Relying Party BFF handlers (hosted mode)
//!
//! All submodules compile unconditionally; the deployment mode is resolved at
//! runtime from the configuration (see [`crate::config::Config::hosted`]).

pub mod jwt;
pub mod middleware;
pub mod oidc_rp;
pub mod oidc_rs;
