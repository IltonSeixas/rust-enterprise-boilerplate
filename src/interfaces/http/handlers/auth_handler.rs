use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::application::{
    dtos::{LoginRequest, RefreshTokenRequest, RegisterRequest},
    use_cases::{LoginUser, RefreshTokenUseCase, RegisterUser},
};

#[derive(Clone)]
pub struct AuthState {
    pub register: Arc<RegisterUser>,
    pub login: Arc<LoginUser>,
    pub refresh: Arc<RefreshTokenUseCase>,
}

pub async fn register(
    State(state): State<AuthState>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    match state.register.execute(req).await {
        Ok(resp) => (StatusCode::CREATED, Json(resp)).into_response(),
        Err(e) => super::error_response(e),
    }
}

pub async fn login(
    State(state): State<AuthState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    match state.login.execute(req).await {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => super::error_response(e),
    }
}

pub async fn refresh(
    State(state): State<AuthState>,
    Json(req): Json<RefreshTokenRequest>,
) -> impl IntoResponse {
    match state.refresh.execute(req).await {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => super::error_response(e),
    }
}
