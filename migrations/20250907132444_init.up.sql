-- Add up migration script here
BEGIN;

-- ===== Enums =====
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'chat_platform') THEN
    CREATE TYPE chat_platform AS ENUM ('whatsapp', 'telegram');
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'binding_status') THEN
    CREATE TYPE binding_status AS ENUM ('active', 'revoked');
  END IF;
END$$;

-- ===== Tables =====

-- users
CREATE TABLE IF NOT EXISTS users (
  uid UUID PRIMARY KEY,
  email VARCHAR NOT NULL UNIQUE,
  phash VARCHAR NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  start_over_date SMALLINT NOT NULL DEFAULT 0
);

-- expense_groups
CREATE TABLE IF NOT EXISTS expense_groups (
  uid UUID PRIMARY KEY,
  name VARCHAR NOT NULL,
  owner UUID NOT NULL REFERENCES users(uid),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_expense_groups_owner ON expense_groups(owner);

-- categories
CREATE TABLE IF NOT EXISTS categories (
  uid UUID PRIMARY KEY,
  group_uid UUID NOT NULL REFERENCES expense_groups(uid),
  name VARCHAR NOT NULL,
  description TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT uq_categories_group_name UNIQUE (group_uid, name)
);

-- unique pair (uid, group_uid) as composite-FK target
CREATE UNIQUE INDEX IF NOT EXISTS cat_uid_group_unique ON categories(uid, group_uid);
CREATE INDEX IF NOT EXISTS idx_categories_group_uid ON categories(group_uid);

-- categories_aliases
CREATE TABLE IF NOT EXISTS categories_aliases (
  alias_uid UUID PRIMARY KEY,
  group_uid UUID NOT NULL REFERENCES expense_groups(uid),
  alias VARCHAR NOT NULL,
  category_uid UUID NOT NULL,
  CONSTRAINT uq_alias_per_group UNIQUE (group_uid, alias)
);

-- expense_entries
CREATE TABLE IF NOT EXISTS expense_entries (
  uid UUID PRIMARY KEY,
  product VARCHAR NOT NULL,
  price NUMERIC(12,2) NOT NULL,
  created_by VARCHAR NOT NULL, -- freeform user identifier (e.g. email or chat name)
  category_uid UUID NULL,
  group_uid UUID NOT NULL REFERENCES expense_groups(uid),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT ck_entries_price_non_negative CHECK (price >= 0)
);

CREATE INDEX IF NOT EXISTS idx_entries_group_uid ON expense_entries(group_uid);
CREATE INDEX IF NOT EXISTS idx_entries_category_uid ON expense_entries(category_uid);
CREATE INDEX IF NOT EXISTS idx_entries_created_at ON expense_entries(created_at);
CREATE INDEX IF NOT EXISTS idx_entries_group_created_at ON expense_entries(group_uid, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_entries_category_created_at ON expense_entries(category_uid, created_at);

-- budgets
CREATE TABLE IF NOT EXISTS budgets (
  uid UUID PRIMARY KEY,
  group_uid UUID NOT NULL REFERENCES expense_groups(uid),
  category_uid UUID NOT NULL,
  amount NUMERIC(12,2) NOT NULL,
  period_year INT,
  period_month INT,
  CONSTRAINT ck_budgets_amount_non_negative CHECK (amount >= 0),
  CONSTRAINT ck_budgets_month_range CHECK (period_month IS NULL OR (period_month BETWEEN 1 AND 12))
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_budgets_group_cat_period
  ON budgets(group_uid, category_uid, period_year, period_month);

CREATE INDEX IF NOT EXISTS idx_budgets_group_period
  ON budgets(group_uid, period_year, period_month);

CREATE INDEX IF NOT EXISTS idx_budgets_group_uid ON budgets(group_uid);
CREATE INDEX IF NOT EXISTS idx_budgets_category_uid ON budgets(category_uid);

-- chat_bind_requests
CREATE TABLE IF NOT EXISTS chat_bind_requests (
  id UUID PRIMARY KEY,
  platform chat_platform NOT NULL,
  p_uid VARCHAR NOT NULL,
  nonce VARCHAR NOT NULL, -- store HASH server-side
  user_uid UUID NULL,     -- filled after login/identification
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_bind_req_platform_puid ON chat_bind_requests(platform, p_uid);
CREATE UNIQUE INDEX IF NOT EXISTS uq_bind_req_nonce ON chat_bind_requests(nonce);
CREATE INDEX IF NOT EXISTS idx_bind_req_expires_at ON chat_bind_requests(expires_at);

-- chat_bindings
CREATE TABLE IF NOT EXISTS chat_bindings (
  id UUID PRIMARY KEY,
  group_uid UUID NOT NULL REFERENCES expense_groups(uid),
  platform chat_platform NOT NULL,
  p_uid VARCHAR NOT NULL,
  status binding_status NOT NULL DEFAULT 'active',
  bound_by UUID NOT NULL REFERENCES users(uid),
  bound_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  revoked_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS idx_chat_bindings_group_uid ON chat_bindings(group_uid);
CREATE INDEX IF NOT EXISTS idx_chat_bindings_platform_puid_status ON chat_bindings(platform, p_uid, status);

-- One ACTIVE binding per (platform, p_uid) — partial unique index
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_class c
    JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE c.relname = 'chat_bindings_one_active_per_chat'
  ) THEN
    EXECUTE 'CREATE UNIQUE INDEX chat_bindings_one_active_per_chat
             ON chat_bindings(platform, p_uid)
             WHERE status = ''active'';';
  END IF;
END$$;

-- group_members (optional shared group roles)
CREATE TABLE IF NOT EXISTS group_members (
  id UUID PRIMARY KEY,
  group_uid UUID NOT NULL REFERENCES expense_groups(uid),
  user_uid UUID NOT NULL REFERENCES users(uid),
  role VARCHAR NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT uq_group_member UNIQUE (group_uid, user_uid)
);

CREATE INDEX IF NOT EXISTS idx_group_members_group_uid ON group_members(group_uid);
CREATE INDEX IF NOT EXISTS idx_group_members_user_uid ON group_members(user_uid);

-- ===== Composite FK constraints (same-group enforcement) =====

-- 1) categories_aliases.category_uid must belong to the same group_uid
ALTER TABLE categories_aliases
  ADD CONSTRAINT fk_aliases_cat_same_group
  FOREIGN KEY (category_uid, group_uid)
  REFERENCES categories(uid, group_uid)
  ON UPDATE CASCADE ON DELETE RESTRICT;

CREATE INDEX IF NOT EXISTS idx_aliases_cat_group ON categories_aliases(category_uid, group_uid);
CREATE INDEX IF NOT EXISTS idx_aliases_group_uid ON categories_aliases(group_uid);

-- 2) expense_entries.category_uid must belong to entries.group_uid (if category_uid not null)
--    (composite FK allows category_uid to be NULL without violating constraint)
ALTER TABLE expense_entries
  ADD CONSTRAINT fk_entries_cat_same_group
  FOREIGN KEY (category_uid, group_uid)
  REFERENCES categories(uid, group_uid)
  ON UPDATE CASCADE ON DELETE RESTRICT;

-- 3) budgets.category_uid must belong to budgets.group_uid
ALTER TABLE budgets
  ADD CONSTRAINT fk_budgets_cat_same_group
  FOREIGN KEY (category_uid, group_uid)
  REFERENCES categories(uid, group_uid)
  ON UPDATE CASCADE ON DELETE RESTRICT;

-- ===== updated_at trigger for audit-friendly updates =====

CREATE OR REPLACE FUNCTION touch_updated_at() RETURNS trigger AS $$
BEGIN
  NEW.updated_at := now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Auto-touch on categories and expense_entries
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_trigger WHERE tgname = 'trg_touch_categories_updated_at'
  ) THEN
    CREATE TRIGGER trg_touch_categories_updated_at
      BEFORE UPDATE ON categories
      FOR EACH ROW
      EXECUTE FUNCTION touch_updated_at();
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM pg_trigger WHERE tgname = 'trg_touch_entries_updated_at'
  ) THEN
    CREATE TRIGGER trg_touch_entries_updated_at
      BEFORE UPDATE ON expense_entries
      FOR EACH ROW
      EXECUTE FUNCTION touch_updated_at();
  END IF;
END$$;

COMMIT;
