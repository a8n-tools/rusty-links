-- LINKS-27: detect a significant login-location change and alert the user.
-- `last_login_country` is the ISO-3166-1 alpha-2 code of the user's most recent
-- login (resolved at the edge from the X-IPCountry header), compared against on
-- the next login. `notify_new_location` is the per-user opt-out; TRUE (the
-- default, backfilled onto existing rows) keeps new-location alerts on.
ALTER TABLE users ADD COLUMN last_login_country VARCHAR(2);
ALTER TABLE users ADD COLUMN notify_new_location BOOLEAN NOT NULL DEFAULT TRUE;
