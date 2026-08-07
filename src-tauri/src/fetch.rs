//! Загрузка подписки и декодирование тела ответа.

use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig, STANDARD, URL_SAFE};
use base64::{alphabet, engine::DecodePaddingMode, Engine};
use std::time::Duration;

/// Некоторые панели отдают разный контент в зависимости от User-Agent.
const USER_AGENT: &str = "v2rayNG/1.8.0";

pub const PREFIXES: [&str; 4] = ["vless://", "vmess://", "trojan://", "ss://"];

/// Base64 без паддинга (и обычный, и url-safe) — подписки встречаются в обоих видах.
const NO_PAD: GeneralPurposeConfig = GeneralPurposeConfig::new()
    .with_decode_padding_mode(DecodePaddingMode::Indifferent)
    .with_decode_allow_trailing_bits(true);
const STANDARD_LOOSE: GeneralPurpose = GeneralPurpose::new(&alphabet::STANDARD, NO_PAD);
const URL_SAFE_LOOSE: GeneralPurpose = GeneralPurpose::new(&alphabet::URL_SAFE, NO_PAD);

/// Декодирует base64 в любом из четырёх вариантов (std/url-safe × с паддингом/без).
pub fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.is_empty() {
        return None;
    }
    for engine in [&STANDARD, &URL_SAFE] {
        if let Ok(bytes) = engine.decode(&cleaned) {
            return Some(bytes);
        }
    }
    for engine in [&STANDARD_LOOSE, &URL_SAFE_LOOSE] {
        if let Ok(bytes) = engine.decode(&cleaned) {
            return Some(bytes);
        }
    }
    None
}

/// Ответ уже является готовым списком ссылок?
pub fn looks_like_links(text: &str) -> bool {
    text.lines()
        .any(|l| PREFIXES.iter().any(|p| l.trim_start().starts_with(p)))
}

/// Тело подписки → текст со ссылками (при необходимости через base64).
pub fn decode_body(body: &str) -> String {
    if looks_like_links(body) {
        return body.to_string();
    }
    match decode_base64(body).and_then(|b| String::from_utf8(b).ok()) {
        Some(decoded) if looks_like_links(&decoded) => decoded,
        _ => body.to_string(),
    }
}

pub async fn fetch_subscription(url: &str) -> Result<String, String> {
    let url = url.trim();
    let parsed = url::Url::parse(url).map_err(|_| "Некорректная ссылка подписки".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Ссылка должна начинаться с http:// или https://".into());
    }

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("Не удалось создать HTTP-клиент: {e}"))?;

    let response = client.get(parsed).send().await.map_err(|e| {
        if e.is_timeout() {
            "Превышено время ожидания ответа сервера".to_string()
        } else if e.is_connect() {
            "Не удалось подключиться к серверу подписки — проверьте адрес и интернет".to_string()
        } else {
            format!("Ошибка сети: {e}")
        }
    })?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "Сервер подписки вернул статус {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        ));
    }

    let body = response
        .text()
        .await
        .map_err(|_| "Не удалось прочитать ответ сервера".to_string())?;

    if body.trim().is_empty() {
        return Err("Сервер подписки вернул пустой ответ".into());
    }

    Ok(decode_body(&body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_all_base64_flavours() {
        let raw = "vless://uuid@example.com:443#test\ntrojan://pass@a.b:443#x";
        use base64::engine::general_purpose::{STANDARD_NO_PAD, URL_SAFE_NO_PAD};
        for encoded in [
            STANDARD.encode(raw),
            URL_SAFE.encode(raw),
            STANDARD_NO_PAD.encode(raw),
            URL_SAFE_NO_PAD.encode(raw),
        ] {
            assert_eq!(decode_body(&encoded), raw);
        }
        // уже готовый список остаётся как есть
        assert_eq!(decode_body(raw), raw);
    }
}
