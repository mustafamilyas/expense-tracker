use anyhow::Result;
use expense_tracker::middleware::tier::check_tier_limit;
use expense_tracker::types::SubscriptionTier;
use expense_tracker::{
    db::make_db_pool,
    repos::{
        budget::{BudgetRepo, CreateBudgetDbPayload},
        category::{CategoryRepo, CreateCategoryDbPayload, UpdateCategoryDbPayload},
        expense_group::{CreateExpenseGroupDbPayload, ExpenseGroupRepo},
        subscription::{CreateSubscriptionDbPayload, SubscriptionRepo},
        user::{CreateUserDbPayload, UpdateUserDbPayload, UserRepo},
    },
};
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
    let Some(pool) = ensure_db_pool().await? else {
        return Ok(());
    };
    let mut tx = pool.begin().await?;

    // Create
    let email = format!("test+{}@example.com", Uuid::new_v4());
    let created = UserRepo::create(
        &mut tx,
        CreateUserDbPayload {
            email: email.clone(),
            phash: "hash".into(),
            start_over_date: 1,
        },
    )
    .await?;
    assert_eq!(created.email, email);

    // Get
    let fetched = UserRepo::get(&mut tx, created.uid).await?;
    assert_eq!(fetched.uid, created.uid);

    // Update
    let new_email = format!("updated+{}@example.com", Uuid::new_v4());
    let updated = UserRepo::update(
        &mut tx,
        created.uid,
        UpdateUserDbPayload {
            email: Some(new_email.clone()),
            phash: None,
            start_over_date: Some(2),
        },
    )
    .await?;
    assert_eq!(updated.email, new_email);
    assert_eq!(updated.start_over_date, 2);

    // rollback test data implicitly by dropping tx
    drop(tx);
    Ok(())
}

#[tokio::test]
async fn category_repo_crud_smoke() -> Result<()> {
    let Some(pool) = ensure_db_pool().await? else {
        return Ok(());
    };
    let mut tx = pool.begin().await?;

    // prerequisites: user and group
    let owner = UserRepo::create(
        &mut tx,
        CreateUserDbPayload {
            email: format!("owner+{}@example.com", Uuid::new_v4()),
            phash: "hash".into(),
            start_over_date: 1,
        },
    )
    .await?;
    let group = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: "Test Group".into(),
            owner: owner.uid,
        },
    )
    .await?;

    // Create category
    let category = CategoryRepo::create(
        &mut tx,
        CreateCategoryDbPayload {
            group_uid: group.uid,
            name: "Groceries".into(),
            description: Some("food".into()),
        },
    )
    .await?;
    assert_eq!(category.group_uid, group.uid);

    // Get
    let fetched = CategoryRepo::get(&mut tx, category.uid).await?;
    assert_eq!(fetched.uid, category.uid);

    // Update
    let updated = CategoryRepo::update(
        &mut tx,
        category.uid,
        UpdateCategoryDbPayload {
            name: Some("Supermarket".into()),
            description: None,
        },
    )
    .await?;
    assert_eq!(updated.name, "Supermarket");

    // Delete
    CategoryRepo::delete(&mut tx, category.uid).await?;

    // rollback test data implicitly by dropping tx
    drop(tx);
    Ok(())
}

#[tokio::test]
async fn tier_limits_enforcement_test() -> Result<()> {
    let Some(pool) = ensure_db_pool().await? else {
        return Ok(());
    };
    let mut tx = pool.begin().await?;

    // Create a test user
    let user = UserRepo::create(
        &mut tx,
        CreateUserDbPayload {
            email: format!("tier-test+{}@example.com", Uuid::new_v4()),
            phash: "hash".into(),
            start_over_date: 1,
        },
    )
    .await?;

    // Create a free subscription for the user
    let subscription = SubscriptionRepo::create(
        &mut tx,
        CreateSubscriptionDbPayload {
            user_uid: user.uid,
            tier: SubscriptionTier::Free,
            status: Some("active".to_string()),
            current_period_start: None,
            current_period_end: None,
        },
    )
    .await?;

    // Test group limit (Free tier allows 1 group)
    let group1 = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: "Test Group 1".into(),
            owner: user.uid,
        },
    )
    .await?;

    // This should succeed (1st group)
    assert_eq!(group1.owner, user.uid);

    // Test category limit (Free tier allows 5 categories per group)
    for i in 1..=5 {
        let category = CategoryRepo::create(
            &mut tx,
            CreateCategoryDbPayload {
                group_uid: group1.uid,
                name: format!("Category {}", i),
                description: None,
            },
        )
        .await?;
        assert_eq!(category.group_uid, group1.uid);
    }

    // Test budget limit (Free tier allows 3 budgets per group)
    let category = CategoryRepo::create(
        &mut tx,
        CreateCategoryDbPayload {
            group_uid: group1.uid,
            name: "Budget Test Category".into(),
            description: None,
        },
    )
    .await?;

    for i in 1..=3 {
        let budget = BudgetRepo::create(
            &mut tx,
            CreateBudgetDbPayload {
                group_uid: group1.uid,
                category_uid: category.uid,
                amount: 100.0 * i as f64,
                period_year: None,
                period_month: None,
            },
        )
        .await?;
        assert_eq!(budget.group_uid, group1.uid);
    }

    // Test tier limit checking function
    let limits = subscription.get_tier().limits();

    // Should allow within limits
    check_tier_limit(&subscription, "groups", 1).expect("Should allow 1 group for free tier");

    // Should fail when exceeding limits
    let result = check_tier_limit(&subscription, "groups", 2);
    assert!(result.is_err(), "Should reject 2nd group for free tier");

    // Verify limits are correct
    assert_eq!(limits.max_groups, 1);
    assert_eq!(limits.max_categories_per_group, 5);
    assert_eq!(limits.max_budgets_per_group, 3);
    assert_eq!(limits.max_expenses_per_month, 100);

    // rollback test data implicitly by dropping tx
    drop(tx);
    Ok(())
}
