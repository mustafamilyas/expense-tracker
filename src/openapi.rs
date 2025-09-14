use utoipa::OpenApi;

use crate::{repos as repo, routes, types};

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::users::list_users,
        routes::users::get_user,
        routes::users::create_user,
        routes::users::update_user,
        routes::users::login_user,

        routes::expense_entry::list_expense_entries,
        routes::expense_entry::create_expense_entry,
        routes::expense_entry::get_expense_entry,
        routes::expense_entry::update_expense_entry,
        routes::expense_entry::delete_expense_entry,

        routes::expense_groups::list,
        routes::expense_groups::get,
        routes::expense_groups::create,
        routes::expense_groups::update,
        routes::expense_groups::delete_,

        routes::categories::list,
        routes::categories::get,
        routes::categories::create,
        routes::categories::update,
        routes::categories::delete_,

        routes::categories_aliases::list,
        routes::categories_aliases::get,
        routes::categories_aliases::create,
        routes::categories_aliases::update,
        routes::categories_aliases::delete_,

        routes::budgets::list,
        routes::budgets::get,
        routes::budgets::create,
        routes::budgets::update,
        routes::budgets::delete_,

        routes::chat_bind_requests::list,
        routes::chat_bind_requests::get,
        routes::chat_bind_requests::create,
        routes::chat_bind_requests::update,
        routes::chat_bind_requests::delete_,

        routes::chat_bindings::list,
        routes::chat_bindings::get,
        routes::chat_bindings::create,
        routes::chat_bindings::update,
        routes::chat_bindings::delete_,

        routes::group_members::list,
        routes::group_members::get,
        routes::group_members::create,
        routes::group_members::update,
        routes::group_members::delete_,

        routes::health::health,
        routes::version::version,
    ),
    components(schemas(
        // Repo models
        repo::user::User,
        repo::user::UserRead,
        repo::expense_group::ExpenseGroup,
        repo::category::Category,
        repo::category_alias::CategoryAlias,
        repo::expense_entry::ExpenseEntry,
        repo::budget::Budget,
        repo::chat_bind_request::ChatBindRequest,
        repo::chat_binding::ChatBinding,
        repo::expense_group_member::GroupMember,
        // Route models
        routes::users::CreateUserPayload,
        routes::users::UpdateUserPayload,
        routes::users::LoginUserPayload,
        routes::expense_entry::CreateExpenseEntryPayload,
        routes::expense_groups::CreatePayload,
        routes::expense_groups::UpdatePayload,
        routes::categories::CreatePayload,
        routes::categories::UpdatePayload,
        routes::categories_aliases::CreatePayload,
        routes::categories_aliases::UpdatePayload,
        routes::budgets::CreatePayload,
        routes::budgets::UpdatePayload,
        routes::chat_bind_requests::CreatePayload,
        routes::chat_bind_requests::UpdatePayload,
        routes::chat_bindings::CreatePayload,
        routes::chat_bindings::UpdatePayload,
        routes::group_members::CreatePayload,
        routes::group_members::UpdatePayload,
        routes::version::VersionBody,
        // Common models
        types::DeleteResponse,
    )),
    tags(
        (name = "Users"),
        (name = "Expense Entries"),
        (name = "Expense Groups"),
        (name = "Categories"),
        (name = "Category Aliases"),
        (name = "Budgets"),
        (name = "Chat Bind Requests"),
        (name = "Chat Bindings"),
        (name = "Group Members"),
        (name = "System"),
    )
)]
pub struct ApiDoc;
