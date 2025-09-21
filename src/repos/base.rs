use crate::error::DatabaseError;

pub trait BaseRepo {
    fn get_table_name() -> &'static str;

    fn create_constraint_error(message: impl Into<String>) -> DatabaseError {
        DatabaseError::ConstraintViolation(format!("{}: {}", Self::get_table_name(), message.into()))
    }

    fn create_not_found_error(resource: impl Into<String>) -> DatabaseError {
        DatabaseError::NotFound(format!("{} not found: {}", resource.into(), Self::get_table_name()))
    }
}
