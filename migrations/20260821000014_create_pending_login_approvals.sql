-- LINKS-35: hold a sign-in from a country the account has not used before until
-- the owner approves it from a single-use emailed link.
-- Only the SHA-256 of the emailed token is stored, so a database dump cannot be
-- used to mint or replay an approval link. `consumed_at` is what makes the link
-- single-use: the claim is a conditional UPDATE guarded on it still being NULL.
CREATE TABLE IF NOT EXISTS pending_login_approvals (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  BYTEA       NOT NULL UNIQUE,
    country     VARCHAR(2)  NOT NULL,
    ip          TEXT        NOT NULL,
    device      TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS pending_login_approvals_user_id ON pending_login_approvals(user_id);
CREATE INDEX IF NOT EXISTS pending_login_approvals_expires ON pending_login_approvals(expires_at);
