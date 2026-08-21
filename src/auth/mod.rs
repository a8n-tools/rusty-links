//! Authentication module for Rusty Links.
//!
//! - `jwt`      — HS256 JWT creation/validation (standalone mode)
//! - `location_alert` - new-sign-in-country detection (LINKS-27)
//! - `mailer`   - outbound security notification email
//! - `middleware` — Axum extractors for authentication
//! - `oidc_rs`  — OIDC Resource Server token verifier (hosted mode)
//! - `oidc_rp`  — OIDC Relying Party BFF handlers (hosted mode)
//! - `trusted_proxy` - peer gate for forwarded IP / country headers (LINKS-31)
//!
//! All submodules compile unconditionally; the deployment mode is resolved at
//! runtime from the configuration (see [`crate::config::Config::hosted`]).

pub mod jwt;
pub mod location_alert;
pub mod mailer;
pub mod middleware;
pub mod oidc_rp;
pub mod oidc_rs;
pub mod trusted_proxy;
