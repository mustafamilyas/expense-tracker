-- Revert: Move start_over_date from expense_groups back to users
BEGIN;

-- Add start_over_date column back to users
ALTER TABLE users
ADD COLUMN start_over_date SMALLINT NOT NULL DEFAULT 1;

-- Copy start_over_date from expense_groups back to users
-- For each user, use the start_over_date from their owned groups, or default to 1
UPDATE users
SET start_over_date = COALESCE(
    (SELECT eg.start_over_date
     FROM expense_groups eg
     WHERE eg.owner = users.uid
     LIMIT 1),
    1
);

-- Remove start_over_date column from expense_groups
ALTER TABLE expense_groups
DROP COLUMN start_over_date;

COMMIT;