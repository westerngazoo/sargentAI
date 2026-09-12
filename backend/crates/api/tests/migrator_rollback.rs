//! The rollback property of `fitai_api::run_migrations`.
//!
//! `main` applies migrations at startup and propagates the error, so the
//! migrator's behaviour on an unknown version decides whether the API can be
//! rolled back at all. sqlx defaults to `ignore_missing: false`, which returns
//! `MigrateError::VersionMissing` when the database records a migration the
//! binary does not embed — i.e. exactly the state an image rollback creates.
//! With that default, a rollback means the container never boots again.
//!
//! These tests pin the chosen behaviour in both directions: an unknown *newer*
//! version is tolerated, and a genuine ordering problem still fails.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sqlx::PgPool;

/// Insert a migration row the binary does not know about — the fingerprint of
/// a database that was migrated by a *newer* build than the one now running.
async fn record_phantom_migration(pool: &PgPool, version: i64) {
    sqlx::query(
        "INSERT INTO _sqlx_migrations \
         (version, description, installed_on, success, checksum, execution_time) \
         VALUES ($1, $2, NOW(), TRUE, $3, 0)",
    )
    .bind(version)
    .bind("from a newer image")
    .bind(vec![0_u8; 48])
    .execute(pool)
    .await
    .expect("phantom migration row inserted");
}

/// The property: a schema newer than this binary does not block startup.
///
/// Without `ignore_missing(true)` this call returns `VersionMissing` and, in
/// `main`, aborts the process — which is the outage this test exists to
/// prevent.
#[sqlx::test(migrations = "../../migrations")]
async fn a_newer_schema_does_not_block_startup(pool: PgPool) {
    record_phantom_migration(&pool, 99_999_999_999_999).await;

    fitai_api::run_migrations(&pool)
        .await
        .expect("a version this binary does not know must not stop the API booting");
}

/// Running twice is a no-op, so a restart loop cannot corrupt anything.
#[sqlx::test(migrations = "../../migrations")]
async fn re_running_is_idempotent(pool: PgPool) {
    fitai_api::run_migrations(&pool).await.expect("first run");
    fitai_api::run_migrations(&pool).await.expect("second run");
}

/// Tolerating an unknown version must not tolerate a *modified* one.
///
/// The guard that matters is the checksum: editing an already-applied
/// migration still fails, which is what stops `00004` from ever being
/// rewritten in place.
#[sqlx::test(migrations = "../../migrations")]
async fn a_tampered_migration_still_fails(pool: PgPool) {
    sqlx::query("UPDATE _sqlx_migrations SET checksum = $1 WHERE version = 4")
        .bind(vec![0_u8; 48])
        .execute(&pool)
        .await
        .expect("checksum overwritten");

    let err = fitai_api::run_migrations(&pool)
        .await
        .expect_err("a modified migration must still abort startup");
    assert!(
        matches!(err, sqlx::migrate::MigrateError::VersionMismatch(4)),
        "expected VersionMismatch(4), got {err:?}"
    );
}
