use utoipa::OpenApi;

use crate::{repos as repo, routes, types};

#[derive(OpenApi)]
#[openapi(
    paths(
        // routes::users::list_users,
        routes::users::get_me,
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
        // routes::expense_groups::delete_,

        routes::categories::list,
        routes::categories::get,
        routes::categories::create,
        routes::categories::update,
        routes::categories::delete_,

        routes::categories_aliases::list,
        routes::categories_aliases::create,
        routes::categories_aliases::update,
        routes::categories_aliases::delete_,

        routes::budgets::list,
        routes::budgets::get,
        routes::budgets::create,
        routes::budgets::update,
        routes::budgets::delete_,

        routes::chat_bind_requests::create,

        routes::chat_bindings::accept,

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
        repo::expense_group::UpdateExpenseGroupDbPayload,
        repo::budget::Budget,
        repo::chat_bind_request::ChatBindRequest,
        repo::chat_binding::ChatBinding,
        repo::expense_group_member::GroupMember,
        // Route models
        routes::users::CreateUserPayload,
        routes::users::UpdateUserPayload,
        routes::users::LoginUserPayload,
        routes::users::LoginResponse,
        routes::expense_groups::CreateExpenseGroupPayload,
        routes::expense_entry::CreateExpenseEntryPayload,
        
        routes::categories::CreateCategoryPayload,
        routes::categories::UpdateCategoryPayload,
        routes::categories_aliases::CreateCategoryAliasPayload,
        routes::categories_aliases::UpdateCategoryAliasPayload,
        routes::budgets::CreateBudgetPayload,
        routes::budgets::UpdateBudgetPayload,
        routes::chat_bind_requests::CreateChatBindRequestPayload,
        routes::chat_bindings::AcceptChatBindingPayload,
        routes::group_members::CreateGroupMemberPayload,
        routes::group_members::UpdateGroupMemberPayload,
        routes::version::VersionBody,
        // Auth docs live in docs/auth.md; OpenAPI only declares bearer scheme.
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
    ),
    modifiers(&ApiSecurity)
)]
pub struct ApiDoc;

use utoipa::Modify;

pub struct ApiSecurity;

impl Modify for ApiSecurity {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{HttpAuthScheme, SecurityScheme};
        use utoipa::openapi::security::HttpBuilder;
        let components = openapi.components.get_or_insert_with(Default::default);
        let bearer = SecurityScheme::Http(
            HttpBuilder::new()
                .scheme(HttpAuthScheme::Bearer)
                .bearer_format("JWT")
                .build(),
        );
        components.add_security_scheme("bearerAuth", bearer);
    }
}
