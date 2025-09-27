use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::error::DatabaseError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("not found")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
    #[error("unauthorized")]
    Unauthorized(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg).into_response(),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            AppError::Internal(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("internal error: {}", err),
            )
                .into_response(),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg).into_response(),
        }
    }
}

impl From<DatabaseError> for AppError {
    fn from(err: DatabaseError) -> Self {
        match err {
            DatabaseError::NotFound(msg) => AppError::NotFound(msg),
            DatabaseError::ConstraintViolation(msg) => AppError::BadRequest(msg),
            _ => AppError::Internal(err.into()),
        }
    }
}

impl AppError {
    pub fn from_sqlx_error(err: sqlx::Error, context: &str) -> Self {
        match DatabaseError::from_sqlx_error(err, context) {
            DatabaseError::NotFound(msg) => AppError::NotFound(msg),
            DatabaseError::ConstraintViolation(msg) => AppError::BadRequest(msg),
            db_err => AppError::Internal(db_err.into()),
        }
    }
}

impl From<crate::types::TierError> for AppError {
    fn from(err: crate::types::TierError) -> Self {
        match err {
            crate::types::TierError::LimitExceeded {
                current,
                limit,
                resource_type,
            } => {
                let suggested_tier = match resource_type.as_str() {
                    "groups" => crate::types::SubscriptionTier::Family,
                    "members_per_group" => crate::types::SubscriptionTier::Family,
                    "categories_per_group" => crate::types::SubscriptionTier::Personal,
                    "budgets_per_group" => crate::types::SubscriptionTier::Personal,
                    "expenses_per_month" => crate::types::SubscriptionTier::Personal,
                    _ => crate::types::SubscriptionTier::Personal,
                };

                AppError::BadRequest(format!(
                    "{} limit exceeded: {}/{}. Upgrade to {} for ${:.2}/month to increase your {} limit.",
                    resource_type,
                    current,
                    limit,
                    suggested_tier.display_name(),
                    suggested_tier.price(),
                    resource_type
                ))
            }
            crate::types::TierError::InsufficientTier {
                required_tier,
                current_tier,
            } => AppError::Unauthorized(format!(
                "Feature requires {} tier (you have {}). Upgrade for ${:.2}/month.",
                required_tier.display_name(),
                current_tier.display_name(),
                required_tier.price()
            )),
            crate::types::TierError::SubscriptionExpired => AppError::Unauthorized(
                "Subscription has expired. Please renew your subscription.".to_string(),
            ),
        }
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        AppError::BadRequest(format!("Validation error: {}", err))
    }
}
