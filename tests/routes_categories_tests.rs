use anyhow::Result;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use expense_tracker::{
    app::build_router,
    db::make_db_pool,
    lang::Lang,
    repos::{
        expense_group::{CreateExpenseGroupDbPayload, ExpenseGroupRepo},
        subscription::{CreateSubscriptionDbPayload, SubscriptionRepo},
        user::{CreateUserDbPayload, UserRepo},
    },
    routes::categories::{CreateCategoryPayload, UpdateCategoryPayload},
    types::{AppState, SubscriptionTier},
};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_db() -> Result<PgPool> {
    // Set up test database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/postgres".to_string());
    let pool = make_db_pool(&database_url).await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

async fn create_test_user_and_auth(pool: &PgPool) -> Result<(Uuid, String)> {
    let mut tx = pool.begin().await?;
    let email = format!("test-{}.example.com", Uuid::new_v4());

    let user = UserRepo::create(
        &mut tx,
        CreateUserDbPayload {
            email: email.clone(),
            phash: "test-hash".to_string(),
        },
    )
    .await?;

    // Create a subscription for the user
    let _ = SubscriptionRepo::create(
        &mut tx,
        CreateSubscriptionDbPayload {
            user_uid: user.uid,
            tier: SubscriptionTier::Personal,
            status: Some("active".to_string()),
            current_period_start: Some(chrono::Utc::now()),
            current_period_end: Some(chrono::Utc::now() + chrono::Duration::days(90)),
        },
    )
    .await?;

    tx.commit().await?;

    // Generate JWT token
    let token =
        expense_tracker::auth::encode_web_jwt(user.uid, "test-jwt-secret", 60 * 60 * 24 * 7)
            .map_err(|e| anyhow::anyhow!("Failed to encode JWT: {}", e))?;

    Ok((user.uid, token))
}

async fn create_test_group(pool: &PgPool, user_uid: Uuid) -> Result<Uuid> {
    let mut tx = pool.begin().await?;
    let group = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: "Test Group".to_string(),
            owner: user_uid,
            start_over_date: 1,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(group.uid)
}

#[tokio::test]
async fn test_list_categories() -> Result<()> {
    let pool = setup_test_db().await?;
    let (user_uid, token) = create_test_user_and_auth(&pool).await?;
    let group_uid = create_test_group(&pool, user_uid).await?;

    // Create some test categories
    let mut tx = pool.begin().await?;
    let category1 = expense_tracker::repos::category::CategoryRepo::create(
        &mut tx,
        expense_tracker::repos::category::CreateCategoryDbPayload {
            group_uid,
            name: "Groceries".to_string(),
            description: Some("Food shopping".to_string()),
        },
    )
    .await?;
    let category2 = expense_tracker::repos::category::CategoryRepo::create(
        &mut tx,
        expense_tracker::repos::category::CreateCategoryDbPayload {
            group_uid,
            name: "Transport".to_string(),
            description: None,
        },
    )
    .await?;
    tx.commit().await?;

    let app_state = AppState {
        lang: Lang::from_json("id"),
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let app = build_router(app_state);
    let request = Request::builder()
        .method("GET")
        .uri(format!("/groups/{}/categories", group_uid))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await?.to_bytes();
    let categories: Vec<serde_json::Value> = serde_json::from_slice(&body)?;

    assert_eq!(categories.len(), 2);
    let category_names: Vec<String> = categories
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_string())
        .collect();
    assert!(category_names.contains(&"Groceries".to_string()));
    assert!(category_names.contains(&"Transport".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_get_category() -> Result<()> {
    let pool = setup_test_db().await?;
    let (user_uid, token) = create_test_user_and_auth(&pool).await?;
    let group_uid = create_test_group(&pool, user_uid).await?;

    // Create a test category
    let mut tx = pool.begin().await?;
    let category = expense_tracker::repos::category::CategoryRepo::create(
        &mut tx,
        expense_tracker::repos::category::CreateCategoryDbPayload {
            group_uid,
            name: "Test Category".to_string(),
            description: Some("Test description".to_string()),
        },
    )
    .await?;
    tx.commit().await?;

    let app_state = AppState {
        lang: Lang::from_json("id"),
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let app = build_router(app_state);
    let request = Request::builder()
        .method("GET")
        .uri(format!("/categories/{}", category.uid))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await?.to_bytes();
    let category_response: serde_json::Value = serde_json::from_slice(&body)?;

    assert_eq!(category_response["uid"], category.uid.to_string());
    assert_eq!(category_response["name"], "Test Category");
    assert_eq!(category_response["description"], "Test description");
    assert_eq!(category_response["group_uid"], group_uid.to_string());

    Ok(())
}

#[tokio::test]
async fn test_get_category_not_found() -> Result<()> {
    let pool = setup_test_db().await?;
    let (_user_uid, token) = create_test_user_and_auth(&pool).await?;

    let app_state = AppState {
        lang: Lang::from_json("id"),
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let app = build_router(app_state);
    let fake_uid = Uuid::new_v4();
    let request = Request::builder()
        .method("GET")
        .uri(format!("/categories/{}", fake_uid))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
async fn test_create_category() -> Result<()> {
    let pool = setup_test_db().await?;
    let (user_uid, token) = create_test_user_and_auth(&pool).await?;
    let group_uid = create_test_group(&pool, user_uid).await?;

    let payload = CreateCategoryPayload {
        group_uid,
        name: "New Category".to_string(),
        description: Some("New category description".to_string()),
        alias: None,
    };

    let app_state = AppState {
        lang: Lang::from_json("id"),
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let app = build_router(app_state);
    let request = Request::builder()
        .method("POST")
        .uri("/categories")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(serde_json::to_string(&payload).unwrap()))?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await?.to_bytes();
    let category_response: serde_json::Value = serde_json::from_slice(&body)?;

    assert_eq!(category_response["name"], "New Category");
    assert_eq!(category_response["description"], "New category description");
    assert_eq!(category_response["group_uid"], group_uid.to_string());
    assert!(category_response.get("uid").is_some());

    Ok(())
}

#[tokio::test]
async fn test_update_category() -> Result<()> {
    let pool = setup_test_db().await?;
    let (user_uid, token) = create_test_user_and_auth(&pool).await?;
    let group_uid = create_test_group(&pool, user_uid).await?;

    // Create a test category
    let mut tx = pool.begin().await?;
    let category = expense_tracker::repos::category::CategoryRepo::create(
        &mut tx,
        expense_tracker::repos::category::CreateCategoryDbPayload {
            group_uid,
            name: "Original Name".to_string(),
            description: Some("Original description".to_string()),
        },
    )
    .await?;
    tx.commit().await?;

    let update_payload = UpdateCategoryPayload {
        name: Some("Updated Name".to_string()),
        description: Some("Updated description".to_string()),
        alias: None,
    };

    let app_state = AppState {
        lang: Lang::from_json("id"),
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let app = build_router(app_state);
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/categories/{}", category.uid))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(serde_json::to_string(&update_payload).unwrap()))?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await?.to_bytes();
    let category_response: serde_json::Value = serde_json::from_slice(&body)?;

    assert_eq!(category_response["name"], "Updated Name");
    assert_eq!(category_response["description"], "Updated description");
    assert_eq!(category_response["uid"], category.uid.to_string());

    Ok(())
}

// #[tokio::test]
// async fn test_delete_category() -> Result<()> {
//     let pool = setup_test_db().await?;
//     let (user_uid, token) = create_test_user_and_auth(&pool).await?;
//     let group_uid = create_test_group(&pool, user_uid).await?;

//     // Create a test category
//     let mut tx = pool.begin().await?;
//     let category = expense_tracker::repos::category::CategoryRepo::create(
//         &mut tx,
//         expense_tracker::repos::category::CreateCategoryDbPayload {
//             group_uid,
//             name: "Category to Delete".to_string(),
//             description: None,
//         },
//     )
//     .await?;
//     tx.commit().await?;

//     let app_state = AppState {
//         version: "test".to_string(),
//         db_pool: pool.clone(),
//         jwt_secret: "test-jwt-secret".to_string(),
//         chat_relay_secret: "test-secret".to_string(),
//         messenger_manager: None,
//     };

//     let app = build_router(app_state);
//     let request = Request::builder()
//         .method("DELETE")
//         .uri(format!("/categories/{}", category.uid))
//         .header("authorization", format!("Bearer {}", token))
//         .body(Body::empty())?;

//     let response = app.oneshot(request).await?;
//     assert_eq!(response.status(), StatusCode::OK);

//     // Verify the category is actually deleted
//     let mut tx = pool.begin().await?;
//     let result = expense_tracker::repos::category::CategoryRepo::get(&mut tx, category.uid).await;
//     assert!(result.is_err());

//     Ok(())
// }

#[tokio::test]
async fn test_categories_unauthorized() -> Result<()> {
    let pool = setup_test_db().await?;
    let lang = Lang::from_json("id");
    let app_state = AppState {
        lang,
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let app = build_router(app_state);
    let request = Request::builder()
        .method("GET")
        .uri("/groups/123e4567-e89b-12d3-a456-426614174000/categories")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}
