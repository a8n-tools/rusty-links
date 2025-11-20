-- Create sessions table for user authentication
-- Sessions use secure random tokens and are tied to user accounts

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Add index on user_id for efficient lookup of all sessions for a user
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
