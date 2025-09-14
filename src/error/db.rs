use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection error: {0}")]
    ConnectionError(sqlx::Error),

    #[error("Migration error: {0}")]
    MigrationError(sqlx::migrate::MigrateError),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Concurrent modification detected")]
    ConcurrentModification,
}

impl DatabaseError {
    pub fn from_sqlx_error(error: sqlx::Error, context: &str) -> Self {
        match error {
            sqlx::Error::RowNotFound => DatabaseError::NotFound(context.to_string()),
            sqlx::Error::Database(db_error) => {
                if let Some(code) = db_error.code() {
                    match code.as_ref() {
                        "23505" => DatabaseError::ConstraintViolation(format!(
                            "Unique constraint violation: {}",
                            context
                        )),
                        "23503" => DatabaseError::ConstraintViolation(format!(
                            "Foreign key constraint violation: {}",
                            context
                        )),
                        "23514" => DatabaseError::ConstraintViolation(format!(
                            "Check constraint violation: {}",
                            context
                        )),
                        _ => DatabaseError::ConnectionError(sqlx::Error::Database(db_error)),
                    }
                } else {
                    DatabaseError::ConnectionError(sqlx::Error::Database(db_error))
                }
            }
            _ => DatabaseError::ConnectionError(error),
        }
    }
}

impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        DatabaseError::from_sqlx_error(err, "Database operation failed")
    }
}
