-- Add missing columns to links table to match Rust models

-- Add is_github_repo boolean column
ALTER TABLE links ADD COLUMN IF NOT EXISTS is_github_repo BOOLEAN NOT NULL DEFAULT false;

-- Change logo from BYTEA to TEXT (base64 encoded or URL)
-- First drop the old column if it exists as BYTEA, then add as TEXT
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'links' AND column_name = 'logo' AND data_type = 'bytea'
    ) THEN
        ALTER TABLE links DROP COLUMN logo;
        ALTER TABLE links ADD COLUMN logo TEXT;
    END IF;
END $$;

-- Make path column nullable (it was NOT NULL but model expects Option<String>)
ALTER TABLE links ALTER COLUMN path DROP NOT NULL;

-- Add index for GitHub repo queries
CREATE INDEX IF NOT EXISTS idx_links_is_github ON links(is_github_repo) WHERE is_github_repo = true;
