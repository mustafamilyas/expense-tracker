#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "subscription_tier")]
pub enum SubscriptionTier {
    #[sqlx(rename = "free")]
    Free,
    #[sqlx(rename = "personal")]
    Personal,
    #[sqlx(rename = "family")]
    Family,
    #[sqlx(rename = "team")]
    Team,
    #[sqlx(rename = "enterprise")]
    Enterprise,
}

impl From<String> for SubscriptionTier {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "personal" => SubscriptionTier::Personal,
            "family" => SubscriptionTier::Family,
            "team" => SubscriptionTier::Team,
            "enterprise" => SubscriptionTier::Enterprise,
            _ => SubscriptionTier::Free,
        }
    }
}

impl From<&str> for SubscriptionTier {
    fn from(s: &str) -> Self {
        SubscriptionTier::from(s.to_string())
    }
}

impl SubscriptionTier {
    pub fn limits(&self) -> TierLimits {
        match self {
            SubscriptionTier::Free => TierLimits {
                max_groups: 1,
                max_members_per_group: 1,
                max_categories_per_group: 5,
                max_budgets_per_group: 3,
                max_expenses_per_month: 100,
                data_retention_days: 90,
                advanced_reports: false,
                export_data: false,
                priority_support: false,
                custom_categories: false,
            },
            SubscriptionTier::Personal => TierLimits {
                max_groups: 1,
                max_members_per_group: 2,
                max_categories_per_group: 20,
                max_budgets_per_group: 10,
                max_expenses_per_month: 1000,
                data_retention_days: 365,
                advanced_reports: false,
                export_data: true,
                priority_support: false,
                custom_categories: true,
            },
            SubscriptionTier::Family => TierLimits {
                max_groups: 3,
                max_members_per_group: 10,
                max_categories_per_group: 50,
                max_budgets_per_group: 25,
                max_expenses_per_month: 5000,
                data_retention_days: 365,
                advanced_reports: true,
                export_data: true,
                priority_support: false,
                custom_categories: true,
            },
            SubscriptionTier::Team => TierLimits {
                max_groups: 10,
                max_members_per_group: 50,
                max_categories_per_group: 100,
                max_budgets_per_group: 50,
                max_expenses_per_month: 25000,
                data_retention_days: 730,
                advanced_reports: true,
                export_data: true,
                priority_support: true,
                custom_categories: true,
            },
            SubscriptionTier::Enterprise => TierLimits {
                max_groups: -1, // Unlimited
                max_members_per_group: -1, // Unlimited
                max_categories_per_group: -1, // Unlimited
                max_budgets_per_group: -1, // Unlimited
                max_expenses_per_month: -1, // Unlimited
                data_retention_days: 2555, // ~7 years
                advanced_reports: true,
                export_data: true,
                priority_support: true,
                custom_categories: true,
            },
        }
    }

    pub fn price(&self) -> f64 {
        match self {
            SubscriptionTier::Free => 0.0,
            SubscriptionTier::Personal => 4.99,
            SubscriptionTier::Family => 9.99,
            SubscriptionTier::Team => 19.99,
            SubscriptionTier::Enterprise => 49.99,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SubscriptionTier::Free => "Free",
            SubscriptionTier::Personal => "Personal",
            SubscriptionTier::Family => "Family",
            SubscriptionTier::Team => "Team",
            SubscriptionTier::Enterprise => "Enterprise",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TierLimits {
    pub max_groups: i32, // -1 for unlimited
    pub max_members_per_group: i32,
    pub max_categories_per_group: i32,
    pub max_budgets_per_group: i32,
    pub max_expenses_per_month: i32,
    pub data_retention_days: i32,
    pub advanced_reports: bool,
    pub export_data: bool,
    pub priority_support: bool,
    pub custom_categories: bool,
}

impl TierLimits {
    pub fn check_limit(&self, current: i32, limit: i32) -> Result<(), TierError> {
        if limit == -1 {
            return Ok(()); // Unlimited
        }
        if current >= limit {
            return Err(TierError::LimitExceeded {
                current,
                limit,
                resource_type: "resource".to_string(),
            });
        }
        Ok(())
    }

    pub fn is_near_limit(&self, current: i32, limit: i32) -> bool {
        if limit == -1 {
            return false; // Unlimited
        }
        current >= (limit * 80) / 100 // 80% of limit
    }
}

#[derive(Debug, Clone)]
pub enum TierError {
    LimitExceeded {
        current: i32,
        limit: i32,
        resource_type: String,
    },
    InsufficientTier {
        required_tier: SubscriptionTier,
        current_tier: SubscriptionTier,
    },
    SubscriptionExpired,
}

impl std::fmt::Display for TierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TierError::LimitExceeded { current, limit, resource_type } => {
                write!(f, "Limit exceeded for {}: {}/{}", resource_type, current, limit)
            }
            TierError::InsufficientTier { required_tier, current_tier } => {
                write!(f, "Feature requires {} tier, you have {}", required_tier.display_name(), current_tier.display_name())
            }
            TierError::SubscriptionExpired => {
                write!(f, "Subscription has expired")
            }
        }
    }
}

impl std::error::Error for TierError {}

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use std::sync::Arc;

use crate::messengers::MessengerManager;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::PgPool,
    pub version: String,
    pub jwt_secret: String,
    pub chat_relay_secret: String,
    pub messenger_manager: Option<Arc<MessengerManager>>,
}

#[derive(Serialize, ToSchema)]
pub struct DeleteResponse {
    pub success: bool,
}