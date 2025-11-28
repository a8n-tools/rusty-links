-- Add name column to users table
ALTER TABLE users ADD COLUMN name TEXT NOT NULL DEFAULT '';
