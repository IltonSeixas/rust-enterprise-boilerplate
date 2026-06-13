use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::{
    application::{
        dtos::{
            ChangePasswordRequest as ChangePasswordDto, ChangeRoleRequest as ChangeRoleDto,
            UpdateProfileRequest as UpdateProfileDto, UserResponse as UserResponseDto,
        },
        ports::TokenService,
        use_cases::{ChangePassword, ChangeUserRole, GetUser, UpdateProfile},
    },
    domain::{entities::Role, errors::DomainError, repositories::UserRepository},
};

use super::{
    error::to_status,
    interceptor::authenticate,
    proto::{
        user_service_server::UserService, ChangePasswordRequest, ChangePasswordResponse,
        ChangeRoleRequest, GetMeRequest, UpdateProfileRequest, UserResponse,
    },
};

pub struct UserGrpcService {
    pub get_user: Arc<GetUser>,
    pub update_profile: Arc<UpdateProfile>,
    pub change_password: Arc<ChangePassword>,
    pub change_role: Arc<ChangeUserRole>,
    pub token_svc: Arc<dyn TokenService>,
    pub user_repo: Arc<dyn UserRepository>,
}

fn to_proto_response(dto: UserResponseDto) -> UserResponse {
    UserResponse {
        id: dto.id.to_string(),
        email: dto.email,
        name: dto.name,
        role: dto.role.to_string(),
        is_active: dto.is_active,
        created_at: dto.created_at.to_rfc3339(),
        updated_at: dto.updated_at.to_rfc3339(),
    }
}

#[tonic::async_trait]
impl UserService for UserGrpcService {
    async fn get_me(
        &self,
        request: Request<GetMeRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let caller = authenticate(&request, &self.token_svc, &self.user_repo).await?;

        let resp = self.get_user.execute(caller.id).await.map_err(to_status)?;
        Ok(Response::new(to_proto_response(resp)))
    }

    async fn update_profile(
        &self,
        request: Request<UpdateProfileRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let caller = authenticate(&request, &self.token_svc, &self.user_repo).await?;
        let dto = UpdateProfileDto {
            name: request.into_inner().name,
        };

        let resp = self
            .update_profile
            .execute(caller.id, dto)
            .await
            .map_err(to_status)?;
        Ok(Response::new(to_proto_response(resp)))
    }

    async fn change_password(
        &self,
        request: Request<ChangePasswordRequest>,
    ) -> Result<Response<ChangePasswordResponse>, Status> {
        let caller = authenticate(&request, &self.token_svc, &self.user_repo).await?;
        let req = request.into_inner();
        let dto = ChangePasswordDto {
            current_password: req.current_password,
            new_password: req.new_password,
        };

        self.change_password
            .execute(caller.id, dto)
            .await
            .map_err(to_status)?;
        Ok(Response::new(ChangePasswordResponse {}))
    }

    async fn change_role(
        &self,
        request: Request<ChangeRoleRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let caller = authenticate(&request, &self.token_svc, &self.user_repo).await?;

        if caller.role != Role::Owner {
            return Err(to_status(DomainError::InsufficientPermissions));
        }

        let req = request.into_inner();
        let target_id = req
            .user_id
            .parse()
            .map_err(|_| Status::invalid_argument("invalid user id"))?;
        let role: Role = req
            .role
            .parse()
            .map_err(|_| Status::invalid_argument("invalid role"))?;
        let dto = ChangeRoleDto { role };

        let resp = self
            .change_role
            .execute(caller.id, target_id, dto)
            .await
            .map_err(to_status)?;
        Ok(Response::new(to_proto_response(resp)))
    }
}
