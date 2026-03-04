use crate::config::Config;
use crate::models::Claims;
use actix_web::HttpRequest;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

pub fn create_token(user_id: i64, email: &str, role: &str, config: &Config) -> Result<String, String> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id,
        email: email.to_string(),
        role: role.to_string(),
        exp: now + (config.jwt_expiry_hours as usize) * 3600,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .map_err(|e| format!("Token creation failed: {}", e))
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, String> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| format!("Token verification failed: {}", e))
}

pub fn extract_user_from_request(req: &HttpRequest, config: &Config) -> Result<Claims, String> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or("Missing Authorization header")?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or("Invalid Authorization format")?;

    verify_token(token, &config.jwt_secret)
}

/// Validate email format (basic but effective)
pub fn is_valid_email(email: &str) -> bool {
    let trimmed = email.trim();
    if trimmed.len() < 5 || trimmed.len() > 254 {
        return false;
    }
    let parts: Vec<&str> = trimmed.splitn(2, '@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    if local.is_empty() || local.len() > 64 {
        return false;
    }
    if domain.is_empty() || !domain.contains('.') {
        return false;
    }
    let domain_parts: Vec<&str> = domain.split('.').collect();
    if domain_parts.iter().any(|p| p.is_empty()) {
        return false;
    }
    // Last TLD must be at least 2 chars
    if domain_parts.last().map_or(true, |t| t.len() < 2) {
        return false;
    }
    true
}

/// Validate password strength
pub fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password minimal 8 karakter".to_string());
    }
    if password.len() > 128 {
        return Err("Password maksimal 128 karakter".to_string());
    }
    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    if !has_lower || !has_upper || !has_digit {
        return Err("Password harus mengandung huruf besar, huruf kecil, dan angka".to_string());
    }
    Ok(())
}
