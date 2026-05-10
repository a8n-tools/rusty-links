//! Build-time metadata embedded by `build.rs`.
//!
//! Available on both client (WASM) and server builds so the same constants can
//! drive log lines, the `/api/health` payload, and the UI footer.

pub const VERSION: &str = env!("BUILD_VERSION");
pub const GIT_HASH: &str = env!("BUILD_GIT_HASH");
pub const BUILD_DATE: &str = env!("BUILD_DATE");
