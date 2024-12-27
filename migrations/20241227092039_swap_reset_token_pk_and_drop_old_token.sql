ALTER TABLE
    password_resets DROP CONSTRAINT IF EXISTS password_resets_pkey;

ALTER TABLE
    password_resets
ADD
    CONSTRAINT password_resets_pkey PRIMARY KEY (token_hash);

ALTER TABLE
    password_resets DROP COLUMN IF EXISTS reset_token;