use std::{fs, path::Path};

use anyhow::{Context, Result};
use argon2::{password_hash::{PasswordHasher, SaltString, rand_core::OsRng}, Argon2};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

#[derive(Deserialize, Debug, Clone)]
struct SeedUser {
    uid: Option<Uuid>,
    email: String,
    password: String,
    #[serde(default)]
    start_over_date: Option<i16>,
}

#[derive(Deserialize, Debug, Clone)]
struct SeedExpenseGroup {
    uid: Option<Uuid>,
    name: String,
    owner: Uuid,
}

#[derive(Deserialize, Debug, Clone)]
struct SeedCategory {
    uid: Option<Uuid>,
    group_uid: Uuid,
    name: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct SeedCategoryAlias {
    alias_uid: Option<Uuid>,
    group_uid: Uuid,
    alias: String,
    category_uid: Uuid,
}

#[derive(Deserialize, Debug, Clone)]
struct SeedExpenseEntry {
    uid: Option<Uuid>,
    product: String,
    price: f64,
    group_uid: Uuid,
    #[serde(default)]
    category_uid: Option<Uuid>,
    #[serde(default = "default_created_by")] 
    created_by: String,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>, // optional in JSON; DB defaults will handle if None
    #[serde(default)]
    updated_at: Option<DateTime<Utc>>,
}

fn default_created_by() -> String { "seed".to_string() }

#[derive(Deserialize, Debug, Clone)]
struct SeedBudget {
    uid: Option<Uuid>,
    group_uid: Uuid,
    category_uid: Uuid,
    amount: f64,
    #[serde(default)]
    period_year: Option<i32>,
    #[serde(default)]
    period_month: Option<i32>,
}

#[derive(Deserialize, Debug, Clone)]
struct SeedGroupMember {
    id: Option<Uuid>,
    group_uid: Uuid,
    user_uid: Uuid,
    role: String,
}

#[derive(Deserialize, Debug, Clone)]
struct SeedChatBindRequest {
    id: Option<Uuid>,
    platform: String, // 'whatsapp' | 'telegram'
    p_uid: String,
    nonce: String,
    #[serde(default)]
    user_uid: Option<Uuid>,
    expires_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug, Clone)]
struct SeedChatBinding {
    id: Option<Uuid>,
    group_uid: Uuid,
    platform: String, // 'whatsapp' | 'telegram'
    p_uid: String,
    #[serde(default)]
    status: Option<String>, // 'active' | 'revoked'
    bound_by: Uuid,
    #[serde(default)]
    bound_at: Option<DateTime<Utc>>,
    #[serde(default)]
    revoked_at: Option<DateTime<Utc>>,
}

async fn seed_users(pool: &PgPool, seeds_dir: &Path) -> Result<()> {
    let path = seeds_dir.join("users.json");
    if !path.exists() { return Ok(()); }
    let data = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut users: Vec<SeedUser> = serde_json::from_str(&data).with_context(|| format!("parsing {}", path.display()))?;

    for u in users.iter_mut() {
        let uid = u.uid.unwrap_or_else(Uuid::new_v4);
        let start_over_date = u.start_over_date.unwrap_or(0);
        let salt = SaltString::generate(&mut OsRng);
        let phash = Argon2::default()
            .hash_password(u.password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .to_string();

        sqlx::query(
            r#"INSERT INTO users (uid, email, phash, start_over_date)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT DO NOTHING"#,
        )
        .bind(uid)
        .bind(&u.email)
        .bind(phash)
        .bind(start_over_date)
        .execute(pool)
        .await
        .with_context(|| format!("inserting user {}", u.email))?;
        u.uid = Some(uid);
    }
    Ok(())
}

async fn seed_expense_groups(pool: &PgPool, seeds_dir: &Path) -> Result<()> {
    let path = seeds_dir.join("expense_groups.json");
    if !path.exists() { return Ok(()); }
    let data = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let groups: Vec<SeedExpenseGroup> = serde_json::from_str(&data).with_context(|| format!("parsing {}", path.display()))?;

    for g in groups {
        let uid = g.uid.unwrap_or_else(Uuid::new_v4);
        sqlx::query(
            r#"INSERT INTO expense_groups (uid, name, owner)
               VALUES ($1, $2, $3)
               ON CONFLICT DO NOTHING"#,
        )
        .bind(uid)
        .bind(&g.name)
        .bind(g.owner)
        .execute(pool)
        .await
        .with_context(|| format!("inserting expense_group {}", g.name))?;
    }
    Ok(())
}

async fn seed_categories(pool: &PgPool, seeds_dir: &Path) -> Result<()> {
    let path = seeds_dir.join("categories.json");
    if !path.exists() { return Ok(()); }
    let data = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let cats: Vec<SeedCategory> = serde_json::from_str(&data).with_context(|| format!("parsing {}", path.display()))?;

    for c in cats {
        let uid = c.uid.unwrap_or_else(Uuid::new_v4);
        sqlx::query(
            r#"INSERT INTO categories (uid, group_uid, name, description)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT DO NOTHING"#,
        )
        .bind(uid)
        .bind(c.group_uid)
        .bind(&c.name)
        .bind(&c.description)
        .execute(pool)
        .await
        .with_context(|| format!("inserting category {}", c.name))?;
    }
    Ok(())
}

async fn seed_category_aliases(pool: &PgPool, seeds_dir: &Path) -> Result<()> {
    let path = seeds_dir.join("categories_aliases.json");
    if !path.exists() { return Ok(()); }
    let data = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let aliases: Vec<SeedCategoryAlias> = serde_json::from_str(&data).with_context(|| format!("parsing {}", path.display()))?;

    for a in aliases {
        let alias_uid = a.alias_uid.unwrap_or_else(Uuid::new_v4);
        sqlx::query(
            r#"INSERT INTO categories_aliases (alias_uid, group_uid, alias, category_uid)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT DO NOTHING"#,
        )
        .bind(alias_uid)
        .bind(a.group_uid)
        .bind(&a.alias)
        .bind(a.category_uid)
        .execute(pool)
        .await
        .with_context(|| format!("inserting category alias {}", a.alias))?;
    }
    Ok(())
}

async fn seed_expense_entries(pool: &PgPool, seeds_dir: &Path) -> Result<()> {
    let path = seeds_dir.join("expense_entries.json");
    if !path.exists() { return Ok(()); }
    let data = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let entries: Vec<SeedExpenseEntry> = serde_json::from_str(&data).with_context(|| format!("parsing {}", path.display()))?;

    for e in entries {
        let uid = e.uid.unwrap_or_else(Uuid::new_v4);
        sqlx::query(
            r#"INSERT INTO expense_entries (uid, product, price, created_by, category_uid, group_uid, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, now()), COALESCE($8, now()))
               ON CONFLICT DO NOTHING"#,
        )
        .bind(uid)
        .bind(&e.product)
        .bind(e.price)
        .bind(&e.created_by)
        .bind(e.category_uid)
        .bind(e.group_uid)
        .bind(e.created_at)
        .bind(e.updated_at)
        .execute(pool)
        .await
        .with_context(|| format!("inserting expense entry {}", e.product))?;
    }
    Ok(())
}

async fn seed_budgets(pool: &PgPool, seeds_dir: &Path) -> Result<()> {
    let path = seeds_dir.join("budgets.json");
    if !path.exists() { return Ok(()); }
    let data = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let budgets: Vec<SeedBudget> = serde_json::from_str(&data).with_context(|| format!("parsing {}", path.display()))?;

    for b in budgets {
        let uid = b.uid.unwrap_or_else(Uuid::new_v4);
        sqlx::query(
            r#"INSERT INTO budgets (uid, group_uid, category_uid, amount, period_year, period_month)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT DO NOTHING"#,
        )
        .bind(uid)
        .bind(b.group_uid)
        .bind(b.category_uid)
        .bind(b.amount)
        .bind(b.period_year)
        .bind(b.period_month)
        .execute(pool)
        .await
        .with_context(|| format!("inserting budget {:?}/{:?}", b.period_year, b.period_month))?;
    }
    Ok(())
}

async fn seed_group_members(pool: &PgPool, seeds_dir: &Path) -> Result<()> {
    let path = seeds_dir.join("group_members.json");
    if !path.exists() { return Ok(()); }
    let data = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let members: Vec<SeedGroupMember> = serde_json::from_str(&data).with_context(|| format!("parsing {}", path.display()))?;

    for m in members {
        let id = m.id.unwrap_or_else(Uuid::new_v4);
        sqlx::query(
            r#"INSERT INTO group_members (id, group_uid, user_uid, role)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT DO NOTHING"#,
        )
        .bind(id)
        .bind(m.group_uid)
        .bind(m.user_uid)
        .bind(&m.role)
        .execute(pool)
        .await
        .with_context(|| format!("inserting group member {}", m.user_uid))?;
    }
    Ok(())
}

async fn seed_chat_bind_requests(pool: &PgPool, seeds_dir: &Path) -> Result<()> {
    let path = seeds_dir.join("chat_bind_requests.json");
    if !path.exists() { return Ok(()); }
    let data = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let reqs: Vec<SeedChatBindRequest> = serde_json::from_str(&data).with_context(|| format!("parsing {}", path.display()))?;

    for r in reqs {
        let id = r.id.unwrap_or_else(Uuid::new_v4);
        sqlx::query(
            r#"INSERT INTO chat_bind_requests (id, platform, p_uid, nonce, user_uid, expires_at)
               VALUES ($1, CAST($2 AS chat_platform), $3, $4, $5, $6)
               ON CONFLICT DO NOTHING"#,
        )
        .bind(id)
        .bind(&r.platform)
        .bind(&r.p_uid)
        .bind(&r.nonce)
        .bind(r.user_uid)
        .bind(r.expires_at)
        .execute(pool)
        .await
        .with_context(|| format!("inserting chat_bind_request {}:{}", r.platform, r.p_uid))?;
    }
    Ok(())
}

async fn seed_chat_bindings(pool: &PgPool, seeds_dir: &Path) -> Result<()> {
    let path = seeds_dir.join("chat_bindings.json");
    if !path.exists() { return Ok(()); }
    let data = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let binds: Vec<SeedChatBinding> = serde_json::from_str(&data).with_context(|| format!("parsing {}", path.display()))?;

    for b in binds {
        let id = b.id.unwrap_or_else(Uuid::new_v4);
        sqlx::query(
            r#"INSERT INTO chat_bindings (id, group_uid, platform, p_uid, status, bound_by, bound_at, revoked_at)
               VALUES ($1, $2, CAST($3 AS chat_platform), $4,
                       COALESCE(CAST($5 AS binding_status), 'active'::binding_status),
                       $6, COALESCE($7, now()), $8)
               ON CONFLICT DO NOTHING"#,
        )
        .bind(id)
        .bind(b.group_uid)
        .bind(&b.platform)
        .bind(&b.p_uid)
        .bind(&b.status)
        .bind(b.bound_by)
        .bind(b.bound_at)
        .bind(b.revoked_at)
        .execute(pool)
        .await
        .with_context(|| format!("inserting chat_binding {}:{}", b.platform, b.p_uid))?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Determine DB URL
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/postgres".to_string());

    let seeds_dir = Path::new("seeds");
    if !seeds_dir.exists() {
        anyhow::bail!("seeds directory not found at {}", seeds_dir.display());
    }

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_url)
        .await?;

    // Seed in dependency order
    seed_users(&pool, seeds_dir).await?;
    seed_expense_groups(&pool, seeds_dir).await?;
    seed_categories(&pool, seeds_dir).await?;
    seed_category_aliases(&pool, seeds_dir).await?;
    seed_expense_entries(&pool, seeds_dir).await?;
    seed_budgets(&pool, seeds_dir).await?;
    seed_group_members(&pool, seeds_dir).await?;
    seed_chat_bind_requests(&pool, seeds_dir).await?;
    seed_chat_bindings(&pool, seeds_dir).await?;
    // Chat-related tables use enums; provide empty seeds or extend later if needed.

    println!("Seeding complete.");
    Ok(())
}
