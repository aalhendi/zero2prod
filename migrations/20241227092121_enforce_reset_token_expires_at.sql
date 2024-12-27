ALTER TABLE
    password_resets
ALTER COLUMN
    expires_at
SET
    DEFAULT (NOW() + INTERVAL '1 hour');

UPDATE
    password_resets
SET
    expires_at = NOW() + INTERVAL '1 hour'
WHERE
    expires_at IS NULL;

ALTER TABLE
    password_resets
ALTER COLUMN
    expires_at
SET
    NOT NULL;