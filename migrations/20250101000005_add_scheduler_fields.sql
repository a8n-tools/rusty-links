-- Add scheduler-related fields to links table

-- Track when the scheduler last checked this link
ALTER TABLE links ADD COLUMN last_checked TIMESTAMP WITH TIME ZONE;

-- Add index for efficient scheduler queries (find links that need checking)
CREATE INDEX idx_links_last_checked ON links(last_checked) WHERE last_checked IS NOT NULL;

-- Add index to find links that haven't been checked yet
CREATE INDEX idx_links_unchecked ON links(last_checked) WHERE last_checked IS NULL;
