use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::{auth::{AuthContext, AuthSource}, error::AppError, repos::expense_group::ExpenseGroupRepo};

pub async fn group_guard(
    auth: &AuthContext,
    group_uid: Uuid,
    pool:  &Pool<Postgres>
) -> Result<(), AppError>{
    if matches!(auth.source, AuthSource::Chat) && auth.group_uid != Some(group_uid) {
        return Err(AppError::Unauthorized("Group scope mismatch".into()));
    }
    Ok(if matches!(auth.source, AuthSource::Web) {
        let mut tx = pool.begin().await.map_err(|e| AppError::from(e))?;
        let group = ExpenseGroupRepo::get(&mut tx, group_uid).await?;
        if auth.user_uid != group.owner {
            tx.commit().await.map_err(|e| AppError::from(e))?;
            return Err(AppError::Unauthorized("Not the owner of the group".into()));
        }
    })

}
