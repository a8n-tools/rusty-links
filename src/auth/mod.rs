//! Authentication module for Rusty Links
//!
//! This module provides authentication and session management functionality.
//!
//! # Modules
//!
//! - `session` - Session management with secure cookies
//!
//! Future modules will include:
//! - `middleware` - Authentication middleware for protected routes
//! - `handlers` - Authentication API endpoints (login, logout, etc.)

pub mod session;

// Re-export commonly used types for convenience
pub use session::{
    create_clear_session_cookie, create_session, create_session_cookie, delete_session,
    get_session, get_session_from_cookies,
};
