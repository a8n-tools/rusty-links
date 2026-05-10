-- Track whether a session was created via OIDC (SSO) or another method.
-- Existing sessions default to false (treated as non-OIDC).
ALTER TABLE user_sessions ADD COLUMN auth_via_oidc BOOLEAN NOT NULL DEFAULT false;
