CREATE EXTENSION IF NOT EXISTS pgcrypto;

UPDATE
    password_resets
SET
    token_hash = encode(digest(reset_token, 'sha256'), 'hex')
WHERE
    token_hash IS NULL
    AND reset_token IS NOT NULL;