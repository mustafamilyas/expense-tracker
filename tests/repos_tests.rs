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
        },
    )
    .await?;
    assert_eq!(updated.email, new_email);

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
        },
    )
    .await?;
    let group = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: "Test Group".into(),
            owner: owner.uid,
            start_over_date: 1,
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
async fn category_repo_list_and_count() -> Result<()> {
    let Some(pool) = ensure_db_pool().await? else {
        return Ok(());
    };
    let mut tx = pool.begin().await?;

    // prerequisites: user and groups
    let owner = UserRepo::create(
        &mut tx,
        CreateUserDbPayload {
            email: format!("owner+{}@example.com", Uuid::new_v4()),
            phash: "hash".into(),
        },
    )
    .await?;
    let group1 = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: "Test Group 1".into(),
            owner: owner.uid,
            start_over_date: 1,
        },
    )
    .await?;
    let group2 = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: "Test Group 2".into(),
            owner: owner.uid,
            start_over_date: 1,
        },
    )
    .await?;

    // Create categories in different groups
    let category1 = CategoryRepo::create(
        &mut tx,
        CreateCategoryDbPayload {
            group_uid: group1.uid,
            name: "Groceries".into(),
            description: Some("food".into()),
        },
    )
    .await?;
    let category2 = CategoryRepo::create(
        &mut tx,
        CreateCategoryDbPayload {
            group_uid: group1.uid,
            name: "Transport".into(),
            description: None,
        },
    )
    .await?;
    let category3 = CategoryRepo::create(
        &mut tx,
        CreateCategoryDbPayload {
            group_uid: group2.uid,
            name: "Entertainment".into(),
            description: Some("fun".into()),
        },
    )
    .await?;

    // Test list (should return all categories)
    let all_categories = CategoryRepo::list(&mut tx).await?;
    assert!(all_categories.len() >= 3);
    let our_categories: Vec<_> = all_categories.into_iter()
        .filter(|c| c.uid == category1.uid || c.uid == category2.uid || c.uid == category3.uid)
        .collect();
    assert_eq!(our_categories.len(), 3);

    // Test list_by_group for group1
    let group1_categories = CategoryRepo::list_by_group(&mut tx, group1.uid).await?;
    assert_eq!(group1_categories.len(), 2);
    assert!(group1_categories.iter().any(|c| c.uid == category1.uid));
    assert!(group1_categories.iter().any(|c| c.uid == category2.uid));

    // Test list_by_group for group2
    let group2_categories = CategoryRepo::list_by_group(&mut tx, group2.uid).await?;
    assert_eq!(group2_categories.len(), 1);
    assert_eq!(group2_categories[0].uid, category3.uid);

    // Test count_by_group
    let group1_count = CategoryRepo::count_by_group(&mut tx, group1.uid).await?;
    assert_eq!(group1_count, 2);

    let group2_count = CategoryRepo::count_by_group(&mut tx, group2.uid).await?;
    assert_eq!(group2_count, 1);

    // Test count_by_group for empty group
    let empty_group = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: "Empty Group".into(),
            owner: owner.uid,
            start_over_date: 1,
        },
    )
    .await?;
    let empty_count = CategoryRepo::count_by_group(&mut tx, empty_group.uid).await?;
    assert_eq!(empty_count, 0);

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
            start_over_date: 1,
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

#[tokio::test]
async fn expense_group_repo_crud() -> Result<()> {
    let Some(pool) = ensure_db_pool().await? else {
        return Ok(());
    };
    let mut tx = pool.begin().await?;

    // Create a test user first
    let user = UserRepo::create(
        &mut tx,
        CreateUserDbPayload {
            email: format!("expense-group-owner+{}@example.com", Uuid::new_v4()),
            phash: "hash".into(),
        },
    )
    .await?;

    // Test create
    let group_name = "Test Expense Group";
    let created = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: group_name.into(),
            owner: user.uid,
            start_over_date: 1,
        },
    )
    .await?;
    assert_eq!(created.name, group_name);
    assert_eq!(created.owner, user.uid);
    assert!(!created.uid.is_nil());

    // Test get
    let fetched = ExpenseGroupRepo::get(&mut tx, created.uid).await?;
    assert_eq!(fetched.uid, created.uid);
    assert_eq!(fetched.name, group_name);
    assert_eq!(fetched.owner, user.uid);

    // Test get_all_by_owner
    let user_groups = ExpenseGroupRepo::get_all_by_owner(&mut tx, user.uid).await?;
    assert_eq!(user_groups.len(), 1);
    assert_eq!(user_groups[0].uid, created.uid);

    // Test count_by_owner
    let count = ExpenseGroupRepo::count_by_owner(&mut tx, user.uid).await?;
    assert_eq!(count, 1);

    // Test update
    let new_name = "Updated Expense Group";
    let updated = ExpenseGroupRepo::update(
        &mut tx,
        created.uid,
        expense_tracker::repos::expense_group::UpdateExpenseGroupDbPayload {
            name: Some(new_name.into()),
            start_over_date: None,
        },
    )
    .await?;
    assert_eq!(updated.name, new_name);
    assert_eq!(updated.uid, created.uid);

    // Test list (should include our group)
    let all_groups = ExpenseGroupRepo::list(&mut tx).await?;
    assert!(all_groups.len() >= 1);
    let our_group = all_groups.iter().find(|g| g.uid == created.uid).unwrap();
    assert_eq!(our_group.name, new_name);

    // Test delete
    ExpenseGroupRepo::delete(&mut tx, created.uid).await?;

    // Verify it's gone
    let result = ExpenseGroupRepo::get(&mut tx, created.uid).await;
    assert!(result.is_err());

    // Verify count is now 0
    let count_after_delete = ExpenseGroupRepo::count_by_owner(&mut tx, user.uid).await?;
    assert_eq!(count_after_delete, 0);

    // rollback test data implicitly by dropping tx
    drop(tx);
    Ok(())
}

#[tokio::test]
async fn expense_group_repo_multiple_owners() -> Result<()> {
    let Some(pool) = ensure_db_pool().await? else {
        return Ok(());
    };
    let mut tx = pool.begin().await?;

    // Create two test users
    let user1 = UserRepo::create(
        &mut tx,
        CreateUserDbPayload {
            email: format!("user1+{}@example.com", Uuid::new_v4()),
            phash: "hash".into(),
        },
    )
    .await?;

    let user2 = UserRepo::create(
        &mut tx,
        CreateUserDbPayload {
            email: format!("user2+{}@example.com", Uuid::new_v4()),
            phash: "hash".into(),
        },
    )
    .await?;

    // Create groups for each user
    let group1 = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: "User1 Group".into(),
            owner: user1.uid,
            start_over_date: 1,
        },
    )
    .await?;

    let group2 = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: "User2 Group".into(),
            owner: user2.uid,
            start_over_date: 1,
        },
    )
    .await?;

    // Test get_all_by_owner returns only the correct groups
    let user1_groups = ExpenseGroupRepo::get_all_by_owner(&mut tx, user1.uid).await?;
    assert_eq!(user1_groups.len(), 1);
    assert_eq!(user1_groups[0].uid, group1.uid);

    let user2_groups = ExpenseGroupRepo::get_all_by_owner(&mut tx, user2.uid).await?;
    assert_eq!(user2_groups.len(), 1);
    assert_eq!(user2_groups[0].uid, group2.uid);

    // Test counts
    let user1_count = ExpenseGroupRepo::count_by_owner(&mut tx, user1.uid).await?;
    assert_eq!(user1_count, 1);

    let user2_count = ExpenseGroupRepo::count_by_owner(&mut tx, user2.uid).await?;
    assert_eq!(user2_count, 1);

    // rollback test data implicitly by dropping tx
    drop(tx);
    Ok(())
}
