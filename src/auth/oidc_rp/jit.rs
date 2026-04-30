//! Just-In-Time provisioning for OIDC-authenticated users.

use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::oidc_rs::IdTokenClaims;
use crate::error::AppError;

pub struct ProvisionedUser {
    pub id: Uuid,
    pub is_admin: bool,
    pub session_version: i32,
}

/// Load an existing local user by their SaaS `sub`, or provision one on first login.
///
/// Returns `Forbidden` if the user is suspended or lacks membership access.
pub async fn load_or_provision(
    pool: &PgPool,
    id_claims: &IdTokenClaims,
) -> Result<ProvisionedUser, AppError> {
    let saas_uuid: Uuid = id_claims
        .sub
        .parse()
        .map_err(|_| AppError::Internal("Invalid sub claim in ID token".into()))?;

    let is_admin = id_claims.role.as_deref() == Some("admin");

    // Try to load an existing user first (common path after first login).
    let existing = sqlx::query_as::<_, (Uuid, bool, Option<chrono::DateTime<chrono::Utc>>, i32)>(
        "SELECT id, is_admin, suspended_at, session_version FROM users WHERE saas_user_id = $1",
    )
    .bind(saas_uuid)
    .fetch_optional(pool)
    .await?;

    if let Some((id, _existing_is_admin, suspended_at, session_version)) = existing {
        if suspended_at.is_some() {
            return Err(AppError::Forbidden(
                "Your account has been suspended. Contact support.".into(),
            ));
        }
        if !id_claims.has_member_access.unwrap_or(false) {
            return Err(AppError::Forbidden(
                "An active a8n.tools membership is required to access Rusty Links. \
                 Please upgrade your plan at a8n.tools."
                    .into(),
            ));
        }

        // Sync email and admin flag on every login.
        let email = id_claims.email.as_deref().unwrap_or("");
        sqlx::query("UPDATE users SET email = $1, is_admin = $2 WHERE id = $3")
            .bind(email)
            .bind(is_admin)
            .bind(id)
            .execute(pool)
            .await?;

        return Ok(ProvisionedUser {
            id,
            is_admin,
            session_version,
        });
    }

    // First-time login → check for an existing standalone account to link before provisioning.
    if !id_claims.email_verified.unwrap_or(false) {
        return Err(AppError::Forbidden(
            "Please verify your email on a8n.tools before logging in.".into(),
        ));
    }

    if !id_claims.has_member_access.unwrap_or(false) {
        return Err(AppError::Forbidden(
            "An active a8n.tools membership is required to access Rusty Links. \
             Please upgrade your plan at a8n.tools."
                .into(),
        ));
    }

    let email = id_claims
        .email
        .as_deref()
        .ok_or_else(|| AppError::Internal("ID token missing email claim".into()))?;

    // A standalone account with this email may already exist. Link it to the SaaS identity
    // rather than creating a duplicate, then treat it as an existing user.
    let linked = sqlx::query_as::<_, (Uuid, i32)>(
        "UPDATE users SET saas_user_id = $1, is_admin = $2
         WHERE email = $3 AND saas_user_id IS NULL
         RETURNING id, session_version",
    )
    .bind(saas_uuid)
    .bind(is_admin)
    .bind(email)
    .fetch_optional(pool)
    .await?;

    if let Some((id, session_version)) = linked {
        tracing::info!(
            user_id = %id,
            saas_user_id = %saas_uuid,
            "Linked existing standalone account to SSO identity"
        );
        return Ok(ProvisionedUser { id, is_admin, session_version });
    }

    let name = id_claims
        .name
        .as_deref()
        .unwrap_or_else(|| email.split('@').next().unwrap_or("user"));

    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, is_admin, saas_user_id)
         VALUES ($1, $2, '!sso:no-password', $3, $4, $5)",
    )
    .bind(user_id)
    .bind(email)
    .bind(name)
    .bind(is_admin)
    .bind(saas_uuid)
    .execute(pool)
    .await
    .map_err(|e: sqlx::Error| {
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") {
            AppError::Internal(
                "User provisioning conflict; please try logging in again.".into(),
            )
        } else {
            AppError::Database(e)
        }
    })?;

    tracing::info!(
        user_id = %user_id,
        saas_user_id = %saas_uuid,
        "SSO user JIT-provisioned"
    );

    Ok(ProvisionedUser {
        id: user_id,
        is_admin,
        session_version: 0,
    })
}
