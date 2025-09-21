-- Drop triggers
DROP TRIGGER IF EXISTS update_subscriptions_updated_at ON subscriptions;
DROP TRIGGER IF EXISTS update_user_usage_updated_at ON user_usage;

-- Drop function
DROP FUNCTION IF EXISTS update_updated_at_column();

-- Drop indexes
DROP INDEX IF EXISTS idx_subscriptions_user_uid;
DROP INDEX IF EXISTS idx_subscriptions_status;
DROP INDEX IF EXISTS idx_user_usage_user_uid;
DROP INDEX IF EXISTS idx_user_usage_period;

-- Drop tables
DROP TABLE IF EXISTS user_usage;
DROP TABLE IF EXISTS subscriptions;

-- Drop enum
DROP TYPE IF EXISTS subscription_tier;