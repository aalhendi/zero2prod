-- Column set as optional before becoming mandatory as part of multistep migration.
ALTER TABLE subscriptions
ADD COLUMN status TEXT NULL;