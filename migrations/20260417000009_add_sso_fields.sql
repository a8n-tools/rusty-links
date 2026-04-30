ALTER TABLE users
    ADD COLUMN IF NOT EXISTS saas_user_id UUID,
    ADD COLUMN IF NOT EXISTS suspended_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS session_version INT NOT NULL DEFAULT 0;

CREATE UNIQUE INDEX IF NOT EXISTS users_saas_user_id_unique
    ON users(saas_user_id)
    WHERE saas_user_id IS NOT NULL;
