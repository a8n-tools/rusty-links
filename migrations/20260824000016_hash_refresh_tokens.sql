-- LINKS-59: store only the SHA-256 of a refresh token, matching
-- `user_sessions.session_token_hash` and `pending_login_approvals.token_hash`.
--
-- `refresh_tokens.token` held the value as issued, the one column in this
-- schema a reader could present back as proof. A dump, a backup, or a single
-- SELECT yielded tokens exchangeable for a live session, and a refresh is
-- session continuation rather than a sign-in, so replaying one never trips the
-- LINKS-35 approval gate.
--
-- BACKFILL, not invalidation: the refresh path now looks up SHA-256(presented)
-- instead of the presented value, so hashing the stored column in place leaves
-- every live token still matching its own row. Nobody is signed out by this
-- migration. `sha256(bytea)` is a core built-in (PostgreSQL 11+), so no
-- extension is required, and it produces the same 32 bytes as `Sha256::digest`
-- in `crate::auth::jwt::hash_refresh_token`.
ALTER TABLE refresh_tokens ADD COLUMN IF NOT EXISTS token_hash BYTEA;

UPDATE refresh_tokens SET token_hash = sha256(convert_to(token, 'UTF8')) WHERE token_hash IS NULL;

ALTER TABLE refresh_tokens ALTER COLUMN token_hash SET NOT NULL;

-- The UNIQUE is the lookup's index, as it is on both sibling tables, so the
-- separate `idx_refresh_tokens_token` is not recreated over the hash.
ALTER TABLE refresh_tokens ADD CONSTRAINT refresh_tokens_token_hash_key UNIQUE (token_hash);

-- Takes `idx_refresh_tokens_token` and `refresh_tokens_token_key` with it, so
-- no plaintext token and no index over one is left at rest.
ALTER TABLE refresh_tokens DROP COLUMN IF EXISTS token;
