-- Add down migration script here
BEGIN;

-- Recreate categories_aliases table
CREATE TABLE IF NOT EXISTS categories_aliases (
  alias_uid UUID PRIMARY KEY,
  group_uid UUID NOT NULL REFERENCES expense_groups(uid),
  alias VARCHAR NOT NULL,
  category_uid UUID NOT NULL,
  CONSTRAINT uq_alias_per_group UNIQUE (group_uid, alias)
);

-- Migrate category aliases back to separate table
INSERT INTO categories_aliases (alias_uid, group_uid, alias, category_uid)
SELECT
  gen_random_uuid() as alias_uid,
  c.group_uid,
  c.alias,
  c.uid as category_uid
FROM categories c
WHERE c.alias IS NOT NULL;

-- Add back the composite FK constraint
ALTER TABLE categories_aliases
  ADD CONSTRAINT fk_aliases_cat_same_group
  FOREIGN KEY (category_uid, group_uid)
  REFERENCES categories(uid, group_uid)
  ON UPDATE CASCADE ON DELETE RESTRICT;

-- Add back indexes
CREATE INDEX IF NOT EXISTS idx_aliases_cat_group ON categories_aliases(category_uid, group_uid);
CREATE INDEX IF NOT EXISTS idx_aliases_group_uid ON categories_aliases(group_uid);

-- Remove alias column from categories
ALTER TABLE categories DROP COLUMN IF EXISTS alias;

COMMIT;