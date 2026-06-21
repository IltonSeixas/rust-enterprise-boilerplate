use async_trait::async_trait;
use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    application::ports::{AccessTokenClaims, TokenPair, TokenService},
    domain::{entities::Role, errors::DomainError},
    infrastructure::resilience::{call_with_resilience, CircuitBreaker, RetryPolicy},
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
    redis_breaker: CircuitBreaker,
    retry_policy: RetryPolicy,
}

impl JwtTokenService {
    /// `private_key_pem` and `public_key_pem` must be PKCS#8 PEM-encoded Ed25519 keys,
    /// e.g. generated via `openssl genpkey -algorithm ed25519`.
    pub fn new(
        private_key_pem: &[u8],
        public_key_pem: &[u8],
        access_ttl_seconds: i64,
        refresh_ttl_seconds: i64,
        redis: redis::aio::ConnectionManager,
        redis_breaker: CircuitBreaker,
        retry_policy: RetryPolicy,
    ) -> Result<Self, jsonwebtoken::errors::Error> {
        Ok(Self {
            encoding_key: EncodingKey::from_ed_pem(private_key_pem)?,
            decoding_key: DecodingKey::from_ed_pem(public_key_pem)?,
            access_ttl_seconds,
            refresh_ttl_seconds,
            redis,
            redis_breaker,
            retry_policy,
        })
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

        let access_token = encode(&Header::new(Algorithm::EdDSA), &claims, &self.encoding_key)
            .map_err(|e| DomainError::Repository(e.to_string()))?;

        let refresh_token = Uuid::new_v4().to_string();

        call_with_resilience(&self.redis_breaker, &self.retry_policy, || {
            let mut conn = self.redis.clone();
            let key = Self::redis_key(&refresh_token);
            let value = user_id.to_string();
            let ttl = self.refresh_ttl_seconds as u64;
            async move {
                let _: () = conn
                    .set_ex(key, value, ttl)
                    .await
                    .map_err(|e| DomainError::Repository(e.to_string()))?;
                Ok(())
            }
        })
        .await?;

        Ok(TokenPair {
            access_token,
            refresh_token,
        })
    }

    async fn validate_access_token(&self, token: &str) -> Result<AccessTokenClaims, DomainError> {
        let data = decode::<JwtClaims>(
            token,
            &self.decoding_key,
            &Validation::new(Algorithm::EdDSA),
        )
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
        let stored: Option<String> =
            call_with_resilience(&self.redis_breaker, &self.retry_policy, || {
                let mut conn = self.redis.clone();
                let key = Self::redis_key(token);
                async move {
                    conn.get(key)
                        .await
                        .map_err(|e| DomainError::Repository(e.to_string()))
                }
            })
            .await?;

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
        let key = Self::redis_key(old_token);

        let stored: Option<String> =
            call_with_resilience(&self.redis_breaker, &self.retry_policy, || {
                let mut conn = self.redis.clone();
                let key = key.clone();
                async move {
                    conn.get(key)
                        .await
                        .map_err(|e| DomainError::Repository(e.to_string()))
                }
            })
            .await?;

        if stored.as_deref() != Some(&user_id.to_string()) {
            return Err(DomainError::InvalidCredentials);
        }

        call_with_resilience(&self.redis_breaker, &self.retry_policy, || {
            let mut conn = self.redis.clone();
            let key = key.clone();
            async move {
                let _: () = conn
                    .del(key)
                    .await
                    .map_err(|e| DomainError::Repository(e.to_string()))?;
                Ok(())
            }
        })
        .await?;

        self.generate_pair(user_id, role).await
    }

    async fn revoke_refresh_token(&self, token: &str) -> Result<(), DomainError> {
        call_with_resilience(&self.redis_breaker, &self.retry_policy, || {
            let mut conn = self.redis.clone();
            let key = Self::redis_key(token);
            async move {
                let _: () = conn
                    .del(key)
                    .await
                    .map_err(|e| DomainError::Repository(e.to_string()))?;
                Ok(())
            }
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test-only Ed25519 PKCS#8 key pair — never used outside this module.
    const TEST_PRIVATE_KEY_PEM: &[u8] = b"-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIOEtLVAjn5fXoZr+T7i0ysYGX3wREHMxWkSk/fi1do79
-----END PRIVATE KEY-----
";
    const TEST_PUBLIC_KEY_PEM: &[u8] = b"-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA8L8xpqriQa22NRiU93msMlmGkJJJYp+k8Y9ITlFIJSk=
-----END PUBLIC KEY-----
";

    #[test]
    fn access_token_is_signed_and_verified_with_eddsa() {
        let encoding_key = EncodingKey::from_ed_pem(TEST_PRIVATE_KEY_PEM).unwrap();
        let decoding_key = DecodingKey::from_ed_pem(TEST_PUBLIC_KEY_PEM).unwrap();

        let now = Utc::now().timestamp();
        let claims = JwtClaims {
            sub: Uuid::new_v4().to_string(),
            role: Role::User,
            iat: now,
            exp: now + 900,
        };

        let token = encode(&Header::new(Algorithm::EdDSA), &claims, &encoding_key)
            .expect("token should be signed with EdDSA");

        let decoded =
            decode::<JwtClaims>(&token, &decoding_key, &Validation::new(Algorithm::EdDSA))
                .expect("token signed with the matching private key should verify");

        assert_eq!(decoded.claims.sub, claims.sub);
        assert_eq!(decoded.header.alg, Algorithm::EdDSA);
    }

    #[test]
    fn access_token_signed_with_a_different_key_pair_is_rejected() {
        let encoding_key = EncodingKey::from_ed_pem(TEST_PRIVATE_KEY_PEM).unwrap();

        let other_public_key_pem = b"-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAYohTHzpULkk0AienlYBbqC2uo/qmBiT3T33RvH/0pTE=
-----END PUBLIC KEY-----
";
        let mismatched_decoding_key = DecodingKey::from_ed_pem(other_public_key_pem).unwrap();

        let now = Utc::now().timestamp();
        let claims = JwtClaims {
            sub: Uuid::new_v4().to_string(),
            role: Role::User,
            iat: now,
            exp: now + 900,
        };

        let token = encode(&Header::new(Algorithm::EdDSA), &claims, &encoding_key).unwrap();

        let result = decode::<JwtClaims>(
            &token,
            &mismatched_decoding_key,
            &Validation::new(Algorithm::EdDSA),
        );

        assert!(
            result.is_err(),
            "token must not verify against an unrelated public key"
        );
    }
}
