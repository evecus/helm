//! 认证相关：bcrypt、HS256 JWT、Cookie/Bearer 提取。
//! 对应 Go 版 golang.org/x/crypto/bcrypt + github.com/golang-jwt/jwt/v5。

use axum::body::Body;
use axum::http::{header, Request};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Go 的 time.Time 序列化为 RFC3339，这里用于 MainConfig.CreatedAt。
pub fn now_rfc3339() -> String {
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default()
}

/// bcrypt 默认 cost=10，生成 `$2a$10$...`（与 Go 的 bcrypt.DefaultCost 一致）。
pub fn bcrypt_hash(password: &str) -> Result<String, String> {
    use bcrypt::{hash_with_salt, Version, DEFAULT_COST};
    let salt: [u8; 16] = rand::random();
    let parts = hash_with_salt(password.as_bytes(), DEFAULT_COST, salt)
        .map_err(|e| format!("bcrypt hash failed: {}", e))?;
    Ok(parts.format_for_version(Version::TwoA))
}

pub fn bcrypt_verify(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

fn b64u(data: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(data)
}

fn b64u_decode(s: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(s).ok()
}

fn hmac_sha256(secret: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(secret).expect("HMAC 接受任意长度密钥");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// 生成 token：header `{"alg":"HS256","typ":"JWT"}`、claims `sub` + `exp`（7 天）。
pub fn jwt_sign(secret: &[u8], username: &str) -> String {
    let header = r#"{"alg":"HS256","typ":"JWT"}"#;
    let payload = serde_json::json!({
        "sub": username,
        "exp": now_unix() + 7 * 24 * 3600,
    })
    .to_string();
    let signing_input = format!("{}.{}", b64u(header.as_bytes()), b64u(payload.as_bytes()));
    let sig = hmac_sha256(secret, signing_input.as_bytes());
    format!("{}.{}", signing_input, b64u(&sig))
}

/// 校验签名、alg 与 exp，返回 sub（用户名）。任何失败都返回 None。
pub fn jwt_verify(secret: &[u8], token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let header: Value = serde_json::from_slice(&b64u_decode(parts[0])?).ok()?;
    if header.get("alg").and_then(Value::as_str) != Some("HS256") {
        return None;
    }
    let expected = hmac_sha256(secret, format!("{}.{}", parts[0], parts[1]).as_bytes());
    let sig = b64u_decode(parts[2])?;
    if !ct_eq(&expected, &sig) {
        return None;
    }
    let payload: Value = serde_json::from_slice(&b64u_decode(parts[1])?).ok()?;
    let exp = payload.get("exp")?.as_i64()?;
    if exp <= now_unix() {
        return None;
    }
    payload
        .get("sub")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

/// 从 Cookie 头里取 `token=`（对应 gin 的 c.Cookie("token")）。
pub fn cookie_token(req: &Request<Body>) -> Option<String> {
    let raw = req.headers().get(header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let mut kv = part.trim().splitn(2, '=');
        if kv.next()?.trim() == "token" {
            let v = kv.next().unwrap_or("").trim();
            // Go 的 cookie 值可能被双引号包裹
            let v = v.trim_matches('"');
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// 从 Authorization: Bearer xxx 取 token。
pub fn bearer_token(req: &Request<Body>) -> Option<String> {
    let raw = req.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    raw.strip_prefix("Bearer ").map(|s| s.to_string())
}

/// 认证中间件与 CheckAuth 共用的取 token 顺序：Cookie 优先，再 Bearer。
pub fn extract_token(req: &Request<Body>) -> Option<String> {
    cookie_token(req).or_else(|| bearer_token(req))
}

/// gin SetCookie(name, value, 7*24*3600, "/", "", false, true) 的等价 Header 值。
pub fn login_cookie(token: &str) -> String {
    format!("token={}; Max-Age=604800; Path=/; HttpOnly", token)
}

/// gin SetCookie("token", "", -1, ...) 的等价 Header 值。
pub fn logout_cookie() -> String {
    "token=; Max-Age=0; Path=/; HttpOnly".to_string()
}
