-- Long-lived BFF sessions.  The rl_session cookie value is SHA-256-hashed
-- before storage so a DB leak cannot be used to forge a cookie.
-- session_version is a snapshot of users.session_version at login time;
-- incrementing users.session_version atomically invalidates all existing sessions.
CREATE TABLE IF NOT EXISTS user_sessions (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    session_token_hash  BYTEA       NOT NULL UNIQUE,
    user_id             UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_version     INT         NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at          TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS user_sessions_user_id ON user_sessions(user_id);
CREATE INDEX IF NOT EXISTS user_sessions_expires  ON user_sessions(expires_at);
