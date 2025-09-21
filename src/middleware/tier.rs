use axum::{
    Json,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::{
    auth::AuthContext,
    error::AppError,
    repos::subscription::SubscriptionRepo,
    types::{AppState, SubscriptionTier, TierError},
};

#[derive(Debug)]
pub struct TierCheck {
    pub required_tier: Option<SubscriptionTier>,
    pub resource_type: String,
    pub current_count: Option<i32>,
    pub limit: Option<i32>,
}

impl TierCheck {
    pub fn new(resource_type: impl Into<String>) -> Self {
        Self {
            required_tier: None,
            resource_type: resource_type.into(),
            current_count: None,
            limit: None,
        }
    }

    pub fn require_tier(mut self, tier: SubscriptionTier) -> Self {
        self.required_tier = Some(tier);
        self
    }

    pub fn check_limit(mut self, current: i32, limit: i32) -> Self {
        self.current_count = Some(current);
        self.limit = Some(limit);
        self
    }
}

pub async fn tier_enforcement_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract auth context from request extensions
    let auth = request.extensions().get::<AuthContext>().cloned();

    if let Some(auth) = auth {
        // Get user's subscription
        let mut tx = state
            .db_pool
            .begin()
            .await
            .map_err(|e| AppError::from_sqlx_error(e, "Starting transaction failed"))?;
        let subscription = SubscriptionRepo::get_by_user(&mut tx, auth.user_uid).await;

        match subscription {
            Ok(sub) => {
                // Check if subscription is active
                if sub.status != "active" {
                    return Ok((
                        StatusCode::PAYMENT_REQUIRED,
                        Json(json!({
                            "error": "Subscription inactive",
                            "message": "Your subscription is not active. Please renew your subscription.",
                            "upgrade_url": "/billing/upgrade"
                        })),
                    ).into_response());
                }

                // Check if subscription has expired
                if let Some(end_date) = sub.current_period_end {
                    if end_date < chrono::Utc::now() {
                        return Ok((
                            StatusCode::PAYMENT_REQUIRED,
                            Json(json!({
                                "error": "Subscription expired",
                                "message": "Your subscription has expired. Please renew your subscription.",
                                "upgrade_url": "/billing/upgrade"
                            })),
                        ).into_response());
                    }
                }

                // Store subscription in request extensions for use in handlers
                request.extensions_mut().insert(sub);
            }
            Err(_) => {
                // No subscription found, create free tier subscription
                let free_subscription = SubscriptionRepo::create(
                    &mut tx,
                    crate::repos::subscription::CreateSubscriptionDbPayload {
                        user_uid: auth.user_uid,
                        tier: SubscriptionTier::Free,
                        status: Some("active".to_string()),
                        current_period_start: None,
                        current_period_end: None,
                    },
                )
                .await
                .map_err(|e| AppError::from(e))?;

                request.extensions_mut().insert(free_subscription);
            }
        }

        tx.commit()
            .await
            .map_err(|e| AppError::from_sqlx_error(e, "Committing transaction failed"))?;
    }

    Ok(next.run(request).await)
}

pub fn check_tier_limit(
    subscription: &crate::repos::subscription::Subscription,
    resource_type: &str,
    current_count: i32,
) -> Result<(), TierError> {
    let limits = subscription.get_tier().limits();

    let limit = match resource_type {
        "groups" => limits.max_groups,
        "members_per_group" => limits.max_members_per_group,
        "categories_per_group" => limits.max_categories_per_group,
        "budgets_per_group" => limits.max_budgets_per_group,
        "expenses_per_month" => limits.max_expenses_per_month,
        _ => return Ok(()), // Unknown resource type, allow
    };

    limits
        .check_limit(current_count, limit)
        .map_err(|e| match e {
            TierError::LimitExceeded { current, limit, .. } => TierError::LimitExceeded {
                current,
                limit,
                resource_type: resource_type.to_string(),
            },
            _ => e,
        })
}

pub fn check_feature_access(
    subscription: &crate::repos::subscription::Subscription,
    feature: &str,
) -> Result<(), TierError> {
    let limits = subscription.get_tier().limits();

    let has_access = match feature {
        "advanced_reports" => limits.advanced_reports,
        "export_data" => limits.export_data,
        "priority_support" => limits.priority_support,
        "custom_categories" => limits.custom_categories,
        _ => true, // Unknown feature, allow access
    };

    if !has_access {
        return Err(TierError::InsufficientTier {
            required_tier: SubscriptionTier::Personal, // Default to personal for unknown features
            current_tier: subscription.get_tier(),
        });
    }

    Ok(())
}

pub fn get_upgrade_message(
    subscription: &crate::repos::subscription::Subscription,
    resource_type: &str,
    current_count: i32,
    limit: i32,
) -> serde_json::Value {
    let current_tier_name = subscription.get_tier().display_name();
    let suggested_tier = match resource_type {
        "groups" => SubscriptionTier::Family,
        "members_per_group" => SubscriptionTier::Family,
        "categories_per_group" => SubscriptionTier::Personal,
        "budgets_per_group" => SubscriptionTier::Personal,
        "expenses_per_month" => SubscriptionTier::Personal,
        _ => SubscriptionTier::Personal,
    };

    json!({
        "warning": format!("You've reached {}% of your {} limit", (current_count * 100) / limit, resource_type),
        "current_usage": format!("{}/{}", current_count, limit),
        "current_tier": current_tier_name,
        "suggested_upgrade": suggested_tier.display_name(),
        "upgrade_price": suggested_tier.price(),
        "upgrade_url": "/billing/upgrade",
        "message": format!(
            "Consider upgrading to {} for ${:.2}/month to increase your {} limit.",
            suggested_tier.display_name(),
            suggested_tier.price(),
            resource_type
        )
    })
}
