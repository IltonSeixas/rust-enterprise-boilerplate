use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{
    application::{
        dtos::{ChangePasswordRequest, ChangeRoleRequest, ListUsersQuery, UpdateProfileRequest},
        use_cases::{ChangePassword, ChangeUserRole, GetUser, ListUsers, UpdateProfile},
    },
    domain::entities::Role,
    interfaces::http::middleware::AuthenticatedUser,
};

#[derive(Clone)]
pub struct UserState {
    pub get_user: Arc<GetUser>,
    pub list_users: Arc<ListUsers>,
    pub update_profile: Arc<UpdateProfile>,
    pub change_password: Arc<ChangePassword>,
    pub change_role: Arc<ChangeUserRole>,
}

pub async fn get_me(
    Extension(auth): Extension<AuthenticatedUser>,
    State(state): State<UserState>,
) -> impl IntoResponse {
    match state.get_user.execute(auth.id).await {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => super::error_response(e),
    }
}

pub async fn update_me(
    Extension(auth): Extension<AuthenticatedUser>,
    State(state): State<UserState>,
    Json(req): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    match state.update_profile.execute(auth.id, req).await {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => super::error_response(e),
    }
}

pub async fn change_password(
    Extension(auth): Extension<AuthenticatedUser>,
    State(state): State<UserState>,
    Json(req): Json<ChangePasswordRequest>,
) -> impl IntoResponse {
    match state.change_password.execute(auth.id, req).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => super::error_response(e),
    }
}

pub async fn get_user(
    Extension(auth): Extension<AuthenticatedUser>,
    State(state): State<UserState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    if auth.id != id && !auth.role.can_manage_roles() {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "insufficient permissions" })),
        )
            .into_response();
    }

    match state.get_user.execute(id).await {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => super::error_response(e),
    }
}

pub async fn list_users(
    Extension(auth): Extension<AuthenticatedUser>,
    State(state): State<UserState>,
    Query(query): Query<ListUsersQuery>,
) -> impl IntoResponse {
    if !auth.role.can_manage_roles() {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "insufficient permissions" })),
        )
            .into_response();
    }

    match state.list_users.execute(query).await {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => super::error_response(e),
    }
}

pub async fn change_role(
    Extension(auth): Extension<AuthenticatedUser>,
    State(state): State<UserState>,
    Path(id): Path<Uuid>,
    Json(req): Json<ChangeRoleRequest>,
) -> impl IntoResponse {
    if auth.role != Role::Owner {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "insufficient permissions" })),
        )
            .into_response();
    }

    match state.change_role.execute(auth.id, id, req).await {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => super::error_response(e),
    }
}
