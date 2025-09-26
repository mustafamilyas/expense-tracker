use anyhow::Result;
use axum::{body::Body, http::Request};
use expense_tracker::{
    app::build_router,
    db::make_db_pool,
    repos::user::{CreateUserDbPayload, UserRepo},
    routes::users::{CreateUserPayload, LoginUserPayload, UpdateUserPayload},
    types::AppState,
};
use http_body_util::BodyExt;
use reqwest::StatusCode;
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

#[tokio::test]
async fn test_create_user_success() -> Result<()> {
    let pool = setup_test_db().await?;

    let email = format!("test-{}.example.com", Uuid::new_v4());

    // Test the repository function directly first
    let mut tx = pool.begin().await?;
    let repo_result = UserRepo::create(
        &mut tx,
        CreateUserDbPayload {
            email: email.clone(),
            phash: "test-hash".to_string(),
            start_over_date: 1,
        },
    )
    .await;

    if let Err(ref e) = repo_result {
        println!("Repo Error: {:?}", e);
        return Err(anyhow::anyhow!("Repository create failed"));
    }

    tx.commit().await?;

    // Now test the route handler
    let payload = CreateUserPayload {
        email: format!("route-test-{}.example.com", Uuid::new_v4()),
        password: "password123".to_string(),
        start_over_date: 1,
    };

    let app_state = AppState {
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let result = expense_tracker::routes::users::create_user(
        axum::extract::State(app_state),
        axum::Json(payload),
    )
    .await;
    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.start_over_date, 1);
    assert!(!user.uid.is_nil());

    Ok(())
}

#[tokio::test]
async fn test_create_user_duplicate_email() -> Result<()> {
    let pool = setup_test_db().await?;

    let email = format!("duplicate-{}.example.com", Uuid::new_v4());
    let payload1 = CreateUserPayload {
        email: email.clone(),
        password: "password123".to_string(),
        start_over_date: 1,
    };

    let payload2 = CreateUserPayload {
        email: email,
        password: "password456".to_string(),
        start_over_date: 2,
    };

    let app_state = AppState {
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    // Create first user - should succeed
    let result1 = expense_tracker::routes::users::create_user(
        axum::extract::State(app_state.clone()),
        axum::Json(payload1),
    )
    .await;
    assert!(result1.is_ok());

    // Try to create user with same email - should fail
    let result2 = expense_tracker::routes::users::create_user(
        axum::extract::State(app_state),
        axum::Json(payload2),
    )
    .await;
    assert!(result2.is_err());

    Ok(())
}

#[tokio::test]
async fn test_list_users() -> Result<()> {
    let pool = setup_test_db().await?;

    // Create test users directly in database
    let mut tx = pool.begin().await?;
    let email1 = format!("user1-{}.example.com", Uuid::new_v4());
    let email2 = format!("user2-{}.example.com", Uuid::new_v4());
    UserRepo::create(
        &mut tx,
        CreateUserDbPayload {
            email: email1.clone(),
            phash: "hash1".to_string(),
            start_over_date: 1,
        },
    )
    .await?;
    UserRepo::create(
        &mut tx,
        CreateUserDbPayload {
            email: email2.clone(),
            phash: "hash2".to_string(),
            start_over_date: 2,
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

    let result = expense_tracker::routes::users::list_users(axum::extract::State(app_state)).await;
    assert!(result.is_ok());

    let users = result.unwrap();
    assert!(users.len() >= 2);

    let emails: Vec<String> = users.iter().map(|u| u.email.clone()).collect();
    assert!(emails.contains(&email1));
    assert!(emails.contains(&email2));

    Ok(())
}

#[tokio::test]
async fn test_update_user_success() -> Result<()> {
    let pool = setup_test_db().await?;

    // Create a test user
    let mut tx = pool.begin().await?;
    let email = format!("update-test-{}.example.com", Uuid::new_v4());
    let user = UserRepo::create(
        &mut tx,
        CreateUserDbPayload {
            email: email.clone(),
            phash: "oldhash".to_string(),
            start_over_date: 1,
        },
    )
    .await?;
    tx.commit().await?;

    let new_email = format!("updated-{}.example.com", Uuid::new_v4());
    let payload = UpdateUserPayload {
        email: Some(new_email.clone()),
        password: None,
        start_over_date: Some(15),
    };

    let app_state = AppState {
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let result = expense_tracker::routes::users::update_user(
        axum::extract::State(app_state),
        axum::extract::Path(user.uid),
        axum::Json(payload),
    )
    .await;
    assert!(result.is_ok());
    let updated_user = result.unwrap();
    assert_eq!(updated_user.email, new_email);
    assert_eq!(updated_user.start_over_date, 15);

    Ok(())
}

#[tokio::test]
async fn test_update_user_not_found() -> Result<()> {
    let pool = setup_test_db().await?;

    let payload = UpdateUserPayload {
        email: Some("should-fail@example.com".to_string()),
        password: None,
        start_over_date: Some(1),
    };

    let app_state = AppState {
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let fake_uid = uuid::Uuid::new_v4();
    let result = expense_tracker::routes::users::update_user(
        axum::extract::State(app_state),
        axum::extract::Path(fake_uid),
        axum::Json(payload),
    )
    .await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_login_user_http() -> Result<()> {
    let pool = setup_test_db().await?;

    // Create a test user first
    let email = format!("login-test-{}.example.com", Uuid::new_v4());
    let password = "testpassword123";

    let create_payload = CreateUserPayload {
        email: email.clone(),
        password: password.to_string(),
        start_over_date: 1,
    };

    let app_state = AppState {
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    // Create user via HTTP
    let app = build_router(app_state.clone());
    let request = Request::builder()
        .method("POST")
        .uri("/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&create_payload).unwrap()))?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);

    // Now test login via HTTP
    let login_payload = LoginUserPayload {
        email: email.clone(),
        password: password.to_string(),
    };

    let app = build_router(app_state);
    let request = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&login_payload).unwrap()))?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await?.to_bytes();
    let login_response: serde_json::Value = serde_json::from_slice(&body)?;

    assert!(login_response.get("token").is_some());
    assert!(login_response.get("user").is_some());
    assert_eq!(login_response["user"]["email"], email);

    Ok(())
}

#[tokio::test]
async fn test_login_user_invalid_credentials() -> Result<()> {
    let pool = setup_test_db().await?;

    let app_state = AppState {
        version: "test".to_string(),
        db_pool: pool.clone(),
        jwt_secret: "test-jwt-secret".to_string(),
        chat_relay_secret: "test-secret".to_string(),
        messenger_manager: None,
    };

    let login_payload = LoginUserPayload {
        email: "nonexistent@example.com".to_string(),
        password: "wrongpassword".to_string(),
    };

    let app = build_router(app_state);
    let request = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&login_payload).unwrap()))?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}
