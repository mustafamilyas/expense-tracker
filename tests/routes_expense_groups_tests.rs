use anyhow::Result;
use expense_tracker::{
    app::build_router,
    db::make_db_pool,
    repos::{
        expense_group::{CreateExpenseGroupDbPayload, ExpenseGroupRepo},
        subscription::{CreateSubscriptionDbPayload, SubscriptionRepo},
        user::{CreateUserDbPayload, UserRepo},
    },
    routes::expense_groups::CreateExpenseGroupPayload,
    types::{AppState, SubscriptionTier},
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
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
            start_over_date: 1,
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
    ).await?;

    tx.commit().await?;

    // Generate JWT token
    let token = expense_tracker::auth::encode_web_jwt(user.uid, "test-jwt-secret", 60 * 60 * 24 * 7)
        .map_err(|e| anyhow::anyhow!("Failed to encode JWT: {}", e))?;

    Ok((user.uid, token))
}

#[tokio::test]
async fn test_list_expense_groups() -> Result<()> {
    let pool = setup_test_db().await?;
    let (user_uid, token) = create_test_user_and_auth(&pool).await?;

    // Create some test groups
    let mut tx = pool.begin().await?;
    let _group1 = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: "Test Group 1".to_string(),
            owner: user_uid,
        },
    )
    .await?;
    let _group2 = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: "Test Group 2".to_string(),
            owner: user_uid,
        },
    )
    .await?;
    tx.commit().await?;

    let app_state = AppState {
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let app = build_router(app_state);
    let request = Request::builder()
        .method("GET")
        .uri("/expense-groups")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await?.to_bytes();
    let groups: Vec<serde_json::Value> = serde_json::from_slice(&body)?;

    assert_eq!(groups.len(), 2);
    let group_names: Vec<String> = groups.iter().map(|g| g["name"].as_str().unwrap().to_string()).collect();
    assert!(group_names.contains(&"Test Group 1".to_string()));
    assert!(group_names.contains(&"Test Group 2".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_get_expense_group() -> Result<()> {
    let pool = setup_test_db().await?;
    let (user_uid, token) = create_test_user_and_auth(&pool).await?;

    // Create a test group
    let mut tx = pool.begin().await?;
    let group = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: "Test Group".to_string(),
            owner: user_uid,
        },
    )
    .await?;
    tx.commit().await?;

    let app_state = AppState {
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let app = build_router(app_state);
    let request = Request::builder()
        .method("GET")
        .uri(format!("/expense-groups/{}", group.uid))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await?.to_bytes();
    let group_response: serde_json::Value = serde_json::from_slice(&body)?;

    assert_eq!(group_response["uid"], group.uid.to_string());
    assert_eq!(group_response["name"], "Test Group");
    assert_eq!(group_response["owner"], user_uid.to_string());

    Ok(())
}

#[tokio::test]
async fn test_get_expense_group_not_found() -> Result<()> {
    let pool = setup_test_db().await?;
    let (_user_uid, token) = create_test_user_and_auth(&pool).await?;

    let app_state = AppState {
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
        .uri(format!("/expense-groups/{}", fake_uid))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
async fn test_create_expense_group() -> Result<()> {
    let pool = setup_test_db().await?;
    let (user_uid, token) = create_test_user_and_auth(&pool).await?;

    let payload = CreateExpenseGroupPayload {
        name: "New Test Group".to_string(),
    };

    let app_state = AppState {
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let app = build_router(app_state);
    let request = Request::builder()
        .method("POST")
        .uri("/expense-groups")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&payload).unwrap(),
        ))?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await?.to_bytes();
    let group_response: serde_json::Value = serde_json::from_slice(&body)?;

    assert_eq!(group_response["name"], "New Test Group");
    assert_eq!(group_response["owner"], user_uid.to_string());
    assert!(group_response.get("uid").is_some());

    Ok(())
}

#[tokio::test]
async fn test_update_expense_group() -> Result<()> {
    let pool = setup_test_db().await?;
    let (user_uid, token) = create_test_user_and_auth(&pool).await?;

    // Create a test group
    let mut tx = pool.begin().await?;
    let group = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: "Original Name".to_string(),
            owner: user_uid,
        },
    )
    .await?;
    tx.commit().await?;

    let update_payload = expense_tracker::repos::expense_group::UpdateExpenseGroupDbPayload {
        name: Some("Updated Name".to_string()),
    };

    let app_state = AppState {
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let app = build_router(app_state);
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/expense-groups/{}", group.uid))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&update_payload).unwrap(),
        ))?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await?.to_bytes();
    let group_response: serde_json::Value = serde_json::from_slice(&body)?;

    assert_eq!(group_response["name"], "Updated Name");
    assert_eq!(group_response["uid"], group.uid.to_string());

    Ok(())
}

#[tokio::test]
async fn test_delete_expense_group() -> Result<()> {
    let pool = setup_test_db().await?;
    let (user_uid, token) = create_test_user_and_auth(&pool).await?;

    // Create a test group
    let mut tx = pool.begin().await?;
    let group = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: "Group to Delete".to_string(),
            owner: user_uid,
        },
    )
    .await?;
    tx.commit().await?;

    let app_state = AppState {
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let app = build_router(app_state);
    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/expense-groups/{}", group.uid))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await?.to_bytes();
    let delete_response: serde_json::Value = serde_json::from_slice(&body)?;

    assert_eq!(delete_response["success"], true);

    // Verify the group is actually deleted
    let mut tx = pool.begin().await?;
    let result = ExpenseGroupRepo::get(&mut tx, group.uid).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_expense_groups_unauthorized() -> Result<()> {
    let pool = setup_test_db().await?;

    let app_state = AppState {
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let app = build_router(app_state);
    let request = Request::builder()
        .method("GET")
        .uri("/expense-groups")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}