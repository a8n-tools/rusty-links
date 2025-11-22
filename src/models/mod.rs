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
//! - `category` - Link categories
//!
//! Future modules will include:
//! - `tag` - Link tags
//! - `language` - Programming languages
//! - `license` - Software licenses

pub mod category;
pub mod language;
pub mod license;
pub mod link;
pub mod tag;
pub mod user;

// Re-export commonly used types for convenience
pub use category::{Category, CategoryWithChildren, CreateCategory};
pub use language::Language;
pub use license::License;
pub use link::{CreateLink, Link, LinkSearchParams, LinkWithCategories, UpdateLink};
pub use tag::Tag;
pub use user::{check_user_exists, create_user, find_user_by_email, verify_password, CreateUser, User};
