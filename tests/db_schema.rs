//! Migration coverage: the shapes the SQL files promise, read back from a real
//! database (LINKS-44).
//!
//! `.forgejo/workflows/check.yml` runs this target on its own before the other
//! database legs, so a migration that does not apply to an empty database fails
//! by name here instead of inside an unrelated test. The behaviour these
//! constraints produce (a duplicate token rejected, a deleted user taking its
//! pending rows with it) is covered in `db_login_approval.rs`.

#![cfg(feature = "server")]

mod common;

use std::path::Path;

/// Every file in `migrations/` is applied, not just the ones a test happened to
/// need. `sqlx::migrate!` embeds them at compile time, so a file added without
/// being applied here means it never ran.
#[tokio::test]
async fn every_migration_applies_to_an_empty_database() {
    let pool = common::test_pool().await;

    let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await
        .expect("the sqlx migration ledger must exist");

    let on_disk = std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations"))
        .expect("migrations/ must be readable")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "sql"))
        .count() as i64;

    assert_eq!(
        applied, on_disk,
        "every migration file must be applied to the test database"
    );
}

/// LINKS-35: `token_hash BYTEA NOT NULL UNIQUE`.
#[tokio::test]
async fn pending_login_approvals_token_hash_is_unique() {
    let pool = common::test_pool().await;

    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT a.attname::text
         FROM pg_constraint c
         JOIN pg_class t ON t.oid = c.conrelid
         JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(c.conkey)
         WHERE t.relname = 'pending_login_approvals' AND c.contype = 'u'",
    )
    .fetch_all(&pool)
    .await
    .expect("unique constraints must be readable");

    assert_eq!(
        columns,
        vec!["token_hash".to_string()],
        "token_hash carries the only UNIQUE constraint on pending_login_approvals"
    );
}

/// LINKS-35: `user_id ... REFERENCES users(id) ON DELETE CASCADE`. `c` is what
/// `pg_constraint.confdeltype` records for CASCADE.
#[tokio::test]
async fn pending_login_approvals_cascade_from_users() {
    let pool = common::test_pool().await;

    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT c.confdeltype::text
         FROM pg_constraint c
         JOIN pg_class t ON t.oid = c.conrelid
         WHERE t.relname = 'pending_login_approvals' AND c.contype = 'f'",
    )
    .fetch_all(&pool)
    .await
    .expect("foreign keys must be readable");

    assert_eq!(
        actions,
        vec!["c".to_string()],
        "the users(id) foreign key must be ON DELETE CASCADE"
    );
}

/// LINKS-27/LINKS-33: the alert columns, including the default that keeps
/// alerts on for rows that predate the migration.
#[tokio::test]
async fn users_carry_the_login_location_columns() {
    let pool = common::test_pool().await;

    let row: (String, String, Option<String>) = sqlx::query_as(
        "SELECT column_name::text, is_nullable::text, column_default::text
         FROM information_schema.columns
         WHERE table_name = 'users' AND column_name = 'notify_new_location'",
    )
    .fetch_one(&pool)
    .await
    .expect("users.notify_new_location must exist");

    assert_eq!(row.1, "NO", "notify_new_location is NOT NULL");
    assert_eq!(
        row.2.as_deref(),
        Some("true"),
        "notify_new_location defaults to TRUE, so alerts stay on unless opted out"
    );

    let nullable: String = sqlx::query_scalar(
        "SELECT is_nullable::text FROM information_schema.columns
         WHERE table_name = 'users' AND column_name = 'last_login_country'",
    )
    .fetch_one(&pool)
    .await
    .expect("users.last_login_country must exist");

    assert_eq!(
        nullable, "YES",
        "last_login_country is NULL until the first login records one"
    );
}
