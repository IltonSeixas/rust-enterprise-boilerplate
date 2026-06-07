use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::application::{
    dtos::{AuthResponse as AuthDto, LoginRequest as LoginDto, RefreshTokenRequest as RefreshDto, RegisterRequest as RegisterDto, UserSummary},
    use_cases::{LoginUser, RefreshTokenUseCase, RegisterUser},
};

use super::{
    error::to_status,
    proto::{
        auth_service_server::AuthService, AuthResponse, LoginRequest, RefreshTokenRequest,
        RegisterRequest, UserSummary as UserSummaryProto,
    },
};

pub struct AuthGrpcService {
    pub register: Arc<RegisterUser>,
    pub login: Arc<LoginUser>,
    pub refresh: Arc<RefreshTokenUseCase>,
}

fn to_proto_response(dto: AuthDto) -> AuthResponse {
    AuthResponse {
        access_token: dto.access_token,
        refresh_token: dto.refresh_token,
        user: Some(to_proto_summary(dto.user)),
    }
}

fn to_proto_summary(summary: UserSummary) -> UserSummaryProto {
    UserSummaryProto {
        id: summary.id.to_string(),
        email: summary.email,
        name: summary.name,
        role: summary.role.to_string(),
    }
}

#[tonic::async_trait]
impl AuthService for AuthGrpcService {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<AuthResponse>, Status> {
        let req = request.into_inner();
        let dto = RegisterDto { email: req.email, password: req.password, name: req.name };

        let resp = self.register.execute(dto).await.map_err(to_status)?;
        Ok(Response::new(to_proto_response(resp)))
    }

    async fn login(&self, request: Request<LoginRequest>) -> Result<Response<AuthResponse>, Status> {
        let req = request.into_inner();
        let dto = LoginDto { email: req.email, password: req.password };

        let resp = self.login.execute(dto).await.map_err(to_status)?;
        Ok(Response::new(to_proto_response(resp)))
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<AuthResponse>, Status> {
        let req = request.into_inner();
        let dto = RefreshDto { refresh_token: req.refresh_token };

        let resp = self.refresh.execute(dto).await.map_err(to_status)?;
        Ok(Response::new(to_proto_response(resp)))
    }
}
