-- Create Subscription Tokens Table
CREATE TABLE
    subscription_tokens (
        subscription_token TEXT NOT NULL,
        -- foreign key
        subscriber_id uuid NOT NULL REFERENCES subscriptions (id),
        PRIMARY KEY (subscription_token)
    );