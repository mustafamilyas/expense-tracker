-- Add up migration script here
BEGIN;

-- Add alias column to categories table
ALTER TABLE categories ADD COLUMN alias VARCHAR;

-- Migrate existing aliases to categories (one alias per category, keeping the first one alphabetically)
-- This is a one-time migration to preserve existing data
UPDATE categories
SET alias = sub.alias
FROM (
    SELECT DISTINCT ON (category_uid) category_uid, alias
    FROM categories_aliases
    ORDER BY category_uid, alias
) sub
WHERE categories.uid = sub.category_uid;

-- Drop the categories_aliases table and its constraints
DROP TABLE IF EXISTS categories_aliases CASCADE;

COMMIT;