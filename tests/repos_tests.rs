use anyhow::Result;
use expense_tracker::{db::make_db_pool, repos::{user::{UserRepo, CreateUserPayload, UpdateUserPayload}, expense_group::{ExpenseGroupRepo, CreateExpenseGroupPayload}, category::{CategoryRepo, CreateCategoryPayload, UpdateCategoryPayload}}};
use sqlx::PgPool;
use uuid::Uuid;

async fn ensure_db_pool() -> Result<Option<PgPool>> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Skipping repo tests: DATABASE_URL not set");
            return Ok(None);
        }
    };
    let pool = make_db_pool(&url).await?;
    // Run migrations to ensure schema exists
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(Some(pool))
}

#[tokio::test]
async fn user_repo_crud_smoke() -> Result<()> {
    let Some(pool) = ensure_db_pool().await? else { return Ok(()); };
    let mut tx = pool.begin().await?;

    // Create
    let email = format!("test+{}@example.com", Uuid::new_v4());
    let created = UserRepo::create(&mut tx, CreateUserPayload { email: email.clone(), phash: "hash".into(), start_over_date: 1 }).await?;
    assert_eq!(created.email, email);

    // Get
    let fetched = UserRepo::get(&mut tx, created.uid).await?;
    assert_eq!(fetched.uid, created.uid);

    // Update
    let new_email = format!("updated+{}@example.com", Uuid::new_v4());
    let updated = UserRepo::update(&mut tx, created.uid, UpdateUserPayload { email: Some(new_email.clone()), phash: None, start_over_date: Some(2) }).await?;
    assert_eq!(updated.email, new_email);
    assert_eq!(updated.start_over_date, 2);

    // Delete
    UserRepo::delete(&mut tx, created.uid).await?;

    // rollback test data implicitly by dropping tx
    drop(tx);
    Ok(())
}

#[tokio::test]
async fn category_repo_crud_smoke() -> Result<()> {
    let Some(pool) = ensure_db_pool().await? else { return Ok(()); };
    let mut tx = pool.begin().await?;

    // prerequisites: user and group
    let owner = UserRepo::create(&mut tx, CreateUserPayload { email: format!("owner+{}@example.com", Uuid::new_v4()), phash: "hash".into(), start_over_date: 1 }).await?;
    let group = ExpenseGroupRepo::create(&mut tx, CreateExpenseGroupPayload { name: "Test Group".into(), owner: owner.uid }).await?;

    // Create category
    let category = CategoryRepo::create(&mut tx, CreateCategoryPayload { group_uid: group.uid, name: "Groceries".into(), description: Some("food".into()) }).await?;
    assert_eq!(category.group_uid, group.uid);

    // Get
    let fetched = CategoryRepo::get(&mut tx, category.uid).await?;
    assert_eq!(fetched.uid, category.uid);

    // Update
    let updated = CategoryRepo::update(&mut tx, category.uid, UpdateCategoryPayload { name: Some("Supermarket".into()), description: None }).await?;
    assert_eq!(updated.name, "Supermarket");

    // Delete
    CategoryRepo::delete(&mut tx, category.uid).await?;

    // rollback test data implicitly by dropping tx
    drop(tx);
    Ok(())
}

