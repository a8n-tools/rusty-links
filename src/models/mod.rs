//! Database models for Rusty Links
//!
//! This module contains all database entity models and their associated
//! operations. Each submodule corresponds to a database table or related
//! set of tables.
//!
//! # Modules
//!
//! - `user` - User authentication and management
//! - `link` - Bookmark links
//!
//! Future modules will include:
//! - `category` - Link categories
//! - `tag` - Link tags
//! - `language` - Programming languages
//! - `license` - Software licenses

pub mod link;
pub mod user;

// Re-export commonly used types for convenience
pub use link::{CreateLink, Link, UpdateLink};
pub use user::{check_user_exists, create_user, find_user_by_email, verify_password, CreateUser, User};
