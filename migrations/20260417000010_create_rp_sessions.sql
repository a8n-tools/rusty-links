-- Transient PKCE state for the BFF Authorization Code flow.
-- Rows are deleted on successful callback; abandoned rows are purged by the scheduler.
CREATE TABLE IF NOT EXISTS rp_sessions (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    state          TEXT        NOT NULL UNIQUE,
    nonce          TEXT        NOT NULL,
    code_verifier  TEXT        NOT NULL,
    return_to      TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at     TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS rp_sessions_expires ON rp_sessions(expires_at);
