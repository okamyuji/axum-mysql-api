use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub async fn require_api_key(
    State(expected): State<[u8; 32]>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let actual: [u8; 32] = Sha256::digest(token.as_bytes()).into();
    if bool::from(actual.ct_eq(&expected)) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub fn key_hash(key: &str) -> Result<[u8; 32], &'static str> {
    if key.len() < 32 {
        return Err("API_KEY must contain at least 32 bytes");
    }
    Ok(Sha256::digest(key.as_bytes()).into())
}
