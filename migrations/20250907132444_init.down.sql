-- Add down migration script here
BEGIN;

-- Drop triggers
DROP TRIGGER IF EXISTS trg_touch_entries_updated_at ON expense_entries;
DROP TRIGGER IF EXISTS trg_touch_categories_updated_at ON categories;

-- Drop trigger function
DROP FUNCTION IF EXISTS touch_updated_at();

-- Drop composite FKs
ALTER TABLE budgets DROP CONSTRAINT IF EXISTS fk_budgets_cat_same_group;
ALTER TABLE expense_entries DROP CONSTRAINT IF EXISTS fk_entries_cat_same_group;
ALTER TABLE categories_aliases DROP CONSTRAINT IF EXISTS fk_aliases_cat_same_group;

-- Drop partial unique index
DROP INDEX IF EXISTS chat_bindings_one_active_per_chat;

-- Drop indexes (safe to leave; optional cleanup)
DROP INDEX IF EXISTS idx_group_members_user_uid;
DROP INDEX IF EXISTS idx_group_members_group_uid;
DROP INDEX IF EXISTS idx_chat_bindings_platform_puid_status;
DROP INDEX IF EXISTS idx_chat_bindings_group_uid;
DROP INDEX IF EXISTS uq_bind_req_nonce;
DROP INDEX IF EXISTS idx_bind_req_expires_at;
DROP INDEX IF EXISTS idx_bind_req_platform_puid;
DROP INDEX IF EXISTS idx_budgets_category_uid;
DROP INDEX IF EXISTS idx_budgets_group_uid;
DROP INDEX IF EXISTS idx_budgets_group_period;
DROP INDEX IF EXISTS uq_budgets_group_cat_period;
DROP INDEX IF EXISTS idx_entries_category_created_at;
DROP INDEX IF EXISTS idx_entries_group_created_at;
DROP INDEX IF EXISTS idx_entries_created_at;
DROP INDEX IF EXISTS idx_entries_category_uid;
DROP INDEX IF EXISTS idx_entries_group_uid;
DROP INDEX IF EXISTS idx_aliases_group_uid;
DROP INDEX IF EXISTS idx_aliases_cat_group;
DROP INDEX IF EXISTS idx_categories_group_uid;
DROP INDEX IF EXISTS cat_uid_group_unique;
DROP INDEX IF EXISTS uq_categories_group_name;
DROP INDEX IF EXISTS idx_expense_groups_owner;

-- Drop tables (in FK-safe order)
DROP TABLE IF EXISTS group_members;
DROP TABLE IF EXISTS chat_bindings;
DROP TABLE IF EXISTS chat_bind_requests;
DROP TABLE IF EXISTS budgets;
DROP TABLE IF EXISTS expense_entries;
DROP TABLE IF EXISTS categories_aliases;
DROP TABLE IF EXISTS categories;
DROP TABLE IF EXISTS expense_groups;
DROP TABLE IF EXISTS users;

-- Drop enums
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_type WHERE typname = 'binding_status') THEN
    DROP TYPE binding_status;
  END IF;
  IF EXISTS (SELECT 1 FROM pg_type WHERE typname = 'chat_platform') THEN
    DROP TYPE chat_platform;
  END IF;
END$$;

COMMIT;
