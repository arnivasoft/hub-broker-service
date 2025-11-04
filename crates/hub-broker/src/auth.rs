use common::{BranchId, TenantId, QualifiedBranchId, Result, Error};
use crate::storage::Storage;
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use axum::{
    extract::State,
    Json,
    http::StatusCode,
};
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub tenant_id: String,
    pub branch_id: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenRequest {
    pub tenant_id: String,
    pub branch_id: String,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
    pub expires_at: i64,
}

/// Generate JWT token for authenticated branch
pub async fn generate_token(
    State(state): State<crate::server::AppState>,
    Json(request): Json<TokenRequest>,
) -> std::result::Result<Json<TokenResponse>, StatusCode> {
    let tenant_id = TenantId::new(request.tenant_id);
    let branch_id = BranchId::new(request.branch_id);

    // Authenticate
    match authenticate_branch(&state.storage, &tenant_id, &branch_id, &request.api_key).await {
        Ok(true) => {
            let now = chrono::Utc::now().timestamp();
            let expires_at = now + state.config.security.jwt_expiry_secs;

            let claims = Claims {
                tenant_id: tenant_id.as_str().to_string(),
                branch_id: branch_id.as_str().to_string(),
                exp: expires_at,
                iat: now,
            };

            match encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(state.config.security.jwt_secret.as_bytes()),
            ) {
                Ok(token) => {
                    info!("Generated token for {}:{}", tenant_id, branch_id);
                    Ok(Json(TokenResponse { token, expires_at }))
                }
                Err(e) => {
                    warn!("Failed to encode token: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        _ => {
            warn!("Authentication failed for {}:{}", tenant_id, branch_id);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Authenticate branch with API key
/// CRITICAL: Tenant isolation must be enforced here
pub async fn authenticate_branch(
    storage: &Storage,
    tenant_id: &TenantId,
    branch_id: &BranchId,
    api_key: &str,
) -> Result<bool> {
    // 1. Check if tenant exists and is active
    let tenant = storage.get_tenant(tenant_id).await?;
    if tenant.status != common::TenantStatus::Active {
        warn!("Tenant {} is not active", tenant_id);
        return Ok(false);
    }

    // 2. Verify branch belongs to this tenant
    let branch = storage.get_branch(tenant_id, branch_id).await?;
    if branch.tenant_id != *tenant_id {
        // CRITICAL: Prevent cross-tenant access
        warn!("Branch {} does not belong to tenant {}", branch_id, tenant_id);
        return Err(Error::AuthorizationFailed(
            "Branch does not belong to tenant".to_string(),
        ));
    }

    // 3. Verify API key
    let stored_key_hash = storage.get_api_key_hash(tenant_id, branch_id).await?;
    verify_api_key(api_key, &stored_key_hash)
}

/// Verify API key using argon2
fn verify_api_key(api_key: &str, stored_hash: &str) -> Result<bool> {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};

    let parsed_hash = PasswordHash::new(stored_hash)
        .map_err(|e| Error::AuthenticationFailed(format!("Invalid hash: {}", e)))?;

    match Argon2::default().verify_password(api_key.as_bytes(), &parsed_hash) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Validate JWT token and extract claims
pub fn validate_token(token: &str, secret: &str) -> Result<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| Error::AuthenticationFailed(format!("Invalid token: {}", e)))
}

/// Hash API key using argon2
pub fn hash_api_key(api_key: &str) -> Result<String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(api_key.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| Error::Internal(format!("Failed to hash: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let api_key = "test_key_12345";
        let hash = hash_api_key(api_key).unwrap();
        assert!(verify_api_key(api_key, &hash).unwrap());
        assert!(!verify_api_key("wrong_key", &hash).unwrap());
    }
}
