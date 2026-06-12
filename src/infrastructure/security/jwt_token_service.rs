use async_trait::async_trait;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    application::ports::{AccessTokenClaims, TokenPair, TokenService},
    domain::{entities::Role, errors::DomainError},
};

#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    sub: String,
    role: Role,
    exp: i64,
    iat: i64,
}

pub struct JwtTokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_ttl_seconds: i64,
    refresh_ttl_seconds: i64,
    redis: redis::aio::ConnectionManager,
}

impl JwtTokenService {
    pub fn new(
        secret: &str,
        access_ttl_seconds: i64,
        refresh_ttl_seconds: i64,
        redis: redis::aio::ConnectionManager,
    ) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            access_ttl_seconds,
            refresh_ttl_seconds,
            redis,
        }
    }

    fn redis_key(token: &str) -> String {
        format!("refresh:{token}")
    }
}

#[async_trait]
impl TokenService for JwtTokenService {
    async fn generate_pair(&self, user_id: Uuid, role: &Role) -> Result<TokenPair, DomainError> {
        let now = Utc::now().timestamp();
        let claims = JwtClaims {
            sub: user_id.to_string(),
            role: role.clone(),
            iat: now,
            exp: now + self.access_ttl_seconds,
        };

        let access_token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| DomainError::Repository(e.to_string()))?;

        let refresh_token = Uuid::new_v4().to_string();

        let mut conn = self.redis.clone();
        let _: () = conn
            .set_ex(
                Self::redis_key(&refresh_token),
                user_id.to_string(),
                self.refresh_ttl_seconds as u64,
            )
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(TokenPair {
            access_token,
            refresh_token,
        })
    }

    async fn validate_access_token(&self, token: &str) -> Result<AccessTokenClaims, DomainError> {
        let data = decode::<JwtClaims>(token, &self.decoding_key, &Validation::default())
            .map_err(|_| DomainError::InvalidCredentials)?;

        let user_id =
            Uuid::parse_str(&data.claims.sub).map_err(|_| DomainError::InvalidCredentials)?;

        Ok(AccessTokenClaims {
            user_id,
            role: data.claims.role,
        })
    }

    async fn find_user_id_by_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<Uuid>, DomainError> {
        let mut conn = self.redis.clone();
        let stored: Option<String> = conn
            .get(Self::redis_key(token))
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))?;

        match stored {
            Some(raw) => Uuid::parse_str(&raw)
                .map(Some)
                .map_err(|_| DomainError::InvalidCredentials),
            None => Ok(None),
        }
    }

    async fn rotate_refresh_token(
        &self,
        old_token: &str,
        user_id: Uuid,
        role: &Role,
    ) -> Result<TokenPair, DomainError> {
        let mut conn = self.redis.clone();
        let key = Self::redis_key(old_token);

        let stored: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))?;

        if stored.as_deref() != Some(&user_id.to_string()) {
            return Err(DomainError::InvalidCredentials);
        }

        let _: () = conn
            .del(&key)
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))?;

        self.generate_pair(user_id, role).await
    }

    async fn revoke_refresh_token(&self, token: &str) -> Result<(), DomainError> {
        let mut conn = self.redis.clone();
        let _: () = conn
            .del(Self::redis_key(token))
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(())
    }
}
