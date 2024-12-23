CREATE TABLE password_resets (
    reset_token TEXT PRIMARY KEY,
    -- automatically removes all password reset entries for a user when that user is deleted
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ
);