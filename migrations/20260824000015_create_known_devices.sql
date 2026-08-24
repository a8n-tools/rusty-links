-- LINKS-45: hold a sign-in from a device the account has not used before, the
-- second trigger on the LINKS-35 approval gate.
--
-- The device id is minted by the browser and sent with the sign-in request;
-- only its SHA-256 is stored, matching how `pending_login_approvals.token_hash`
-- stores its token, so a database dump yields nothing replayable.
--
-- BACKFILL: deliberately none. The table is created empty, so every account
-- that exists on the deploy running this migration has zero known devices.
-- "No recorded devices" is read as the account's baseline and is never new (see
-- `location_alert::is_new_device`), exactly as a NULL `last_login_country` is,
-- so the first sign-in after this migration RECORDS a device instead of being
-- held. Reading it the other way would hold every account at once on deploy
-- day, including the operator's, with the approval mail as the only way back
-- in. The same rule covers a first-ever sign-in, which has no stored id by
-- definition.
CREATE TABLE IF NOT EXISTS known_devices (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id        UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id_hash BYTEA       NOT NULL,
    first_seen_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, device_id_hash)
);

CREATE INDEX IF NOT EXISTS known_devices_user_id ON known_devices(user_id);

-- The device the held sign-in submitted, promoted into `known_devices` only
-- when the owner approves. That promotion is what makes approving terminate:
-- the emailed link is usually opened in another browser, so the held one is
-- only ever recorded from the id it sent with the sign-in. Nullable: a row
-- written before this migration has none, and so does a hold whose client sent
-- no id, which simply records no device on approval.
ALTER TABLE pending_login_approvals ADD COLUMN IF NOT EXISTS device_id_hash BYTEA;

-- Which trigger held the sign-in, so the page and the mail say why rather than
-- always naming the country. Every row that predates this column is a LINKS-35
-- country hold, which is what the default records.
ALTER TABLE pending_login_approvals
    ADD COLUMN IF NOT EXISTS reason TEXT NOT NULL DEFAULT 'new_country';

-- A device-only hold has no country to record when nothing resolves one, which
-- is the default deployment (no geoblock edge, no TRUSTED_PROXY_CIDRS), so the
-- column can no longer be NOT NULL.
ALTER TABLE pending_login_approvals ALTER COLUMN country DROP NOT NULL;
