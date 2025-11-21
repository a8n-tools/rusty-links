-- Add consecutive_failures column for tracking link health check failures
ALTER TABLE links ADD COLUMN consecutive_failures INTEGER NOT NULL DEFAULT 0;
