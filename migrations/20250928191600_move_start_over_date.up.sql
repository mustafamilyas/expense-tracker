-- Move start_over_date from users to expense_groups
BEGIN;

-- Add start_over_date column to expense_groups
ALTER TABLE expense_groups
ADD COLUMN start_over_date SMALLINT NOT NULL DEFAULT 1;

-- Copy start_over_date from users to expense_groups for existing groups
-- Use the owner's start_over_date, or default to 1 if null/0
UPDATE expense_groups
SET start_over_date = COALESCE(
    (SELECT CASE WHEN u.start_over_date > 0 THEN u.start_over_date ELSE 1 END
     FROM users u WHERE u.uid = expense_groups.owner),
    1
);

-- Remove start_over_date column from users
ALTER TABLE users
DROP COLUMN start_over_date;

COMMIT;