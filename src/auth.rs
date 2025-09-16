use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::http::header::AUTHORIZATION;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use hmac::{Hmac, Mac};
use http_body_util::BodyExt as _; // for collect()
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tracing::info;
use uuid::Uuid;

use crate::types::AppState;

#[derive(Clone, Debug)]
pub enum AuthSource {
    Web,
    Chat,
}

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub source: AuthSource,
    pub user_uid: Uuid,
    pub group_uid: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub typ: String,
    pub exp: usize,
}

pub fn encode_web_jwt(user_uid: Uuid, secret: &str, ttl_seconds: u64) -> anyhow::Result<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let claims = Claims {
        sub: user_uid.to_string(),
        typ: "web".to_string(),
        exp: (now + ttl_seconds) as usize,
    };
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

fn is_public_path(path: &str) -> bool {
    matches!(
        path,
        "/health" | "/version" | "/auth/login" | "/auth/register" | "/api-doc/openapi.json"
    ) || path.starts_with("/docs")
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    info!(
        "Auth middleware checking path: {} {:?}",
        path,
        req.headers().get(AUTHORIZATION)
    );
    if is_public_path(path) {
        return Ok(next.run(req).await);
    }

    // Extract AuthContext in handlers using `axum::extract::Extension<AuthContext>`.

    // 1) Try Bearer JWT (web)
    if let Some(authz) = req.headers().get(AUTHORIZATION) {
        if let Ok(val) = authz.to_str() {
            if let Some(token) = val.strip_prefix("Bearer ") {
                let mut validation = Validation::new(Algorithm::HS256);
                validation.validate_exp = true;
                match decode::<Claims>(
                    token,
                    &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
                    &validation,
                ) {
                    Ok(data) if data.claims.typ == "web" => {
                        if let Ok(user_uid) = Uuid::parse_str(&data.claims.sub) {
                            req.extensions_mut().insert(AuthContext {
                                source: AuthSource::Web,
                                user_uid,
                                group_uid: None,
                            });
                            return Ok(next.run(req).await);
                        }
                    }
                    _ => {
                        return Err(StatusCode::UNAUTHORIZED);
                    }
                }
            }
        }
    }

    // 2) Try Chat relay signature
    let sig_hdr = req
        .headers()
        .get("X-Relay-Signature")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let binding_hdr = req
        .headers()
        .get("X-Chat-Binding")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    if let (Some(sig_hdr), Some(binding_hdr)) = (sig_hdr, binding_hdr) {
        // collect body for HMAC verification and restore it
        let (parts, body) = req.into_parts();
        let bytes = body
            .collect()
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .to_bytes();
        let mut req2 = Request::from_parts(parts, Body::from(bytes.clone()));

        // Expect format: sha256=<hex>
        let calc = {
            let mut mac = Hmac::<Sha256>::new_from_slice(state.chat_relay_secret.as_bytes())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            mac.update(&bytes);
            let tag = mac.finalize().into_bytes();
            hex::encode(tag)
        };
        let presented = sig_hdr.strip_prefix("sha256=").unwrap_or("");
        if presented != calc {
            return Err(StatusCode::UNAUTHORIZED);
        }

        // Load binding and ensure active
        let binding_id = match Uuid::parse_str(&binding_hdr) {
            Ok(id) => id,
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        };

        let mut tx = state
            .db_pool
            .begin()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let binding = crate::repos::chat_binding::ChatBindingRepo::get(&mut tx, binding_id)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        tx.commit()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if binding.status != "active" || binding.revoked_at.is_some() {
            return Err(StatusCode::UNAUTHORIZED);
        }

        // Attach group-scoped context attributed to the user who bound it
        req2.extensions_mut().insert(AuthContext {
            source: AuthSource::Chat,
            user_uid: binding.bound_by,
            group_uid: Some(binding.group_uid),
        });
        return Ok(next.run(req2).await);
    }

    Err(StatusCode::UNAUTHORIZED)
}
