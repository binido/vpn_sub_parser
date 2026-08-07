//! Парсеры ссылок vless / vmess / trojan / ss.

use crate::fetch::decode_base64;
use base64::{engine::general_purpose::STANDARD, Engine};
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

/// Символы, которые не нужно экранировать в собранных ссылках.
const KEEP: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    /// uuid для vless/vmess, пароль для trojan/ss
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Host-заголовок для ws/http
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alter_id: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Server {
    pub protocol: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub raw: String,
    pub details: Details,
}

fn pct(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .into_owned()
}

fn non_empty(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

fn query_map(url: &Url) -> HashMap<String, String> {
    url.query_pairs()
        .map(|(k, v)| (k.to_lowercase(), v.into_owned()))
        .collect()
}

fn get(q: &HashMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| q.get(*k).and_then(|v| non_empty(v)))
}

fn parse_url(link: &str, proto: &str) -> Result<Url, String> {
    Url::parse(link).map_err(|_| format!("Не удалось разобрать {proto}-ссылку"))
}

fn host_of(url: &Url, proto: &str) -> Result<String, String> {
    url.host_str()
        .map(|h| h.to_string())
        .ok_or_else(|| format!("В {proto}-ссылке отсутствует адрес сервера"))
}

fn tag(url: &Url, fallback: &str) -> String {
    url.fragment()
        .map(pct)
        .and_then(|s| non_empty(&s))
        .unwrap_or_else(|| fallback.to_string())
}

pub fn parse_vless(link: &str) -> Result<Server, String> {
    let url = parse_url(link, "vless")?;
    let id = non_empty(&pct(url.username())).ok_or("В vless-ссылке отсутствует UUID")?;
    let address = host_of(&url, "vless")?;
    let port = url.port().unwrap_or(443);
    let q = query_map(&url);

    Ok(Server {
        protocol: "vless".into(),
        name: tag(&url, &address),
        address,
        port,
        raw: link.to_string(),
        details: Details {
            id: Some(id),
            encryption: Some(get(&q, &["encryption"]).unwrap_or_else(|| "none".into())),
            flow: get(&q, &["flow"]),
            security: Some(get(&q, &["security"]).unwrap_or_else(|| "none".into())),
            sni: get(&q, &["sni", "peer"]),
            fingerprint: get(&q, &["fp"]),
            public_key: get(&q, &["pbk"]),
            short_id: get(&q, &["sid"]),
            network: Some(get(&q, &["type"]).unwrap_or_else(|| "tcp".into())),
            path: get(&q, &["path"]),
            host: get(&q, &["host"]),
            service_name: get(&q, &["servicename"]),
            alpn: get(&q, &["alpn"]),
            ..Default::default()
        },
    })
}

/// vmess:// — это base64 от JSON-конфига.
pub fn parse_vmess(link: &str) -> Result<Server, String> {
    let payload = link.trim_start_matches("vmess://");
    let json = decode_base64(payload)
        .and_then(|b| String::from_utf8(b).ok())
        .ok_or("Не удалось декодировать base64 в vmess-ссылке")?;
    let v: serde_json::Value =
        serde_json::from_str(&json).map_err(|_| "Внутри vmess-ссылки не JSON")?;

    // поля vmess бывают и строками, и числами
    let s = |key: &str| -> Option<String> {
        match v.get(key) {
            Some(serde_json::Value::String(s)) => non_empty(s),
            Some(serde_json::Value::Number(n)) => Some(n.to_string()),
            _ => None,
        }
    };

    let address = s("add").ok_or("В vmess-ссылке отсутствует адрес сервера")?;
    let port = s("port")
        .and_then(|p| p.parse::<u16>().ok())
        .ok_or("Некорректный порт в vmess-ссылке")?;
    let security = s("tls").unwrap_or_else(|| "none".into());

    Ok(Server {
        protocol: "vmess".into(),
        name: s("ps").unwrap_or_else(|| address.clone()),
        address,
        port,
        raw: link.to_string(),
        details: Details {
            id: s("id"),
            encryption: Some(s("scy").unwrap_or_else(|| "auto".into())),
            security: Some(if security.is_empty() {
                "none".into()
            } else {
                security
            }),
            sni: s("sni"),
            fingerprint: s("fp"),
            network: Some(s("net").unwrap_or_else(|| "tcp".into())),
            path: s("path"),
            host: s("host"),
            alpn: s("alpn"),
            alter_id: s("aid").and_then(|a| a.parse().ok()),
            ..Default::default()
        },
    })
}

pub fn parse_trojan(link: &str) -> Result<Server, String> {
    let url = parse_url(link, "trojan")?;
    let password = non_empty(&pct(url.username())).ok_or("В trojan-ссылке отсутствует пароль")?;
    let address = host_of(&url, "trojan")?;
    let port = url.port().unwrap_or(443);
    let q = query_map(&url);

    Ok(Server {
        protocol: "trojan".into(),
        name: tag(&url, &address),
        address,
        port,
        raw: link.to_string(),
        details: Details {
            id: Some(password),
            security: Some(get(&q, &["security"]).unwrap_or_else(|| "tls".into())),
            sni: get(&q, &["sni", "peer"]),
            fingerprint: get(&q, &["fp"]),
            network: Some(get(&q, &["type"]).unwrap_or_else(|| "tcp".into())),
            path: get(&q, &["path"]),
            host: get(&q, &["host"]),
            alpn: get(&q, &["alpn"]),
            ..Default::default()
        },
    })
}

/// Два формата: `ss://base64(method:pass)@host:port#name` и `ss://base64(method:pass@host:port)#name`.
pub fn parse_shadowsocks(link: &str) -> Result<Server, String> {
    let body = link.trim_start_matches("ss://");
    let (body, fragment) = match body.split_once('#') {
        Some((b, f)) => (b, non_empty(&pct(f))),
        None => (body, None),
    };
    let body = body.split('?').next().unwrap_or(body);

    let normalized = if body.contains('@') {
        // userinfo может быть как base64, так и открытым текстом
        let (userinfo, rest) = body.split_once('@').unwrap();
        let userinfo = decode_base64(userinfo)
            .and_then(|b| String::from_utf8(b).ok())
            .filter(|d| d.contains(':'))
            .unwrap_or_else(|| pct(userinfo));
        format!("{userinfo}@{rest}")
    } else {
        decode_base64(body)
            .and_then(|b| String::from_utf8(b).ok())
            .ok_or("Не удалось декодировать base64 в ss-ссылке")?
    };

    let (userinfo, hostport) = normalized
        .rsplit_once('@')
        .ok_or("Некорректный формат ss-ссылки")?;
    let (method, password) = userinfo
        .split_once(':')
        .ok_or("В ss-ссылке отсутствует метод шифрования")?;
    let (address, port) = hostport
        .rsplit_once(':')
        .ok_or("В ss-ссылке отсутствует порт")?;
    let port: u16 = port
        .trim()
        .parse()
        .map_err(|_| "Некорректный порт в ss-ссылке")?;
    let address = address.trim().to_string();

    Ok(Server {
        protocol: "shadowsocks".into(),
        name: fragment.unwrap_or_else(|| address.clone()),
        address,
        port,
        raw: link.to_string(),
        details: Details {
            id: Some(password.to_string()),
            method: Some(method.to_string()),
            network: Some("tcp".into()),
            security: Some("none".into()),
            ..Default::default()
        },
    })
}

/// Обратная операция: сервер → ссылка. Нужна, когда подписка пришла в виде
/// готовых конфигов Xray, а показать и скопировать надо обычную ссылку.
pub fn to_link(server: &Server) -> String {
    let d = &server.details;
    let name = utf8_percent_encode(&server.name, KEEP).to_string();
    let id = d.id.clone().unwrap_or_default();

    match server.protocol.as_str() {
        "vmess" => {
            let config = serde_json::json!({
                "v": "2",
                "ps": server.name,
                "add": server.address,
                "port": server.port.to_string(),
                "id": id,
                "aid": d.alter_id.unwrap_or(0).to_string(),
                "scy": d.encryption.clone().unwrap_or_else(|| "auto".into()),
                "net": d.network.clone().unwrap_or_else(|| "tcp".into()),
                "type": "none",
                "host": d.host.clone().unwrap_or_default(),
                "path": d.path.clone().unwrap_or_default(),
                "tls": d.security.clone().unwrap_or_default(),
                "sni": d.sni.clone().unwrap_or_default(),
            });
            format!("vmess://{}", STANDARD.encode(config.to_string()))
        }
        "shadowsocks" => {
            let method = d.method.clone().unwrap_or_else(|| "aes-256-gcm".into());
            let userinfo = STANDARD.encode(format!("{method}:{id}"));
            format!(
                "ss://{userinfo}@{}:{}#{name}",
                server.address, server.port
            )
        }
        protocol => {
            let mut query: Vec<String> = Vec::new();
            let mut push = |key: &str, value: Option<&String>| {
                if let Some(v) = value.filter(|v| !v.is_empty()) {
                    query.push(format!("{key}={}", utf8_percent_encode(v, KEEP)));
                }
            };
            if protocol == "vless" {
                push("encryption", d.encryption.as_ref());
                push("flow", d.flow.as_ref());
            }
            push("security", d.security.as_ref());
            push("sni", d.sni.as_ref());
            push("fp", d.fingerprint.as_ref());
            push("pbk", d.public_key.as_ref());
            push("sid", d.short_id.as_ref());
            push("alpn", d.alpn.as_ref());
            push("type", d.network.as_ref());
            push("path", d.path.as_ref());
            push("host", d.host.as_ref());
            push("serviceName", d.service_name.as_ref());
            format!(
                "{protocol}://{}@{}:{}?{}#{name}",
                utf8_percent_encode(&id, KEEP),
                server.address,
                server.port,
                query.join("&")
            )
        }
    }
}

pub fn parse_link(line: &str) -> Result<Server, String> {
    let line = line.trim();
    match line {
        _ if line.starts_with("vless://") => parse_vless(line),
        _ if line.starts_with("vmess://") => parse_vmess(line),
        _ if line.starts_with("trojan://") => parse_trojan(line),
        _ if line.starts_with("ss://") => parse_shadowsocks(line),
        _ => Err("Неизвестный протокол".into()),
    }
}

/// Разбирает текст подписки, молча пропуская строки, которые разобрать не вышло.
pub fn parse_subscription(text: &str) -> Vec<Server> {
    text.lines().filter_map(|l| parse_link(l).ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_protocol() {
        let vless = "vless://11111111-2222-3333-4444-555555555555@example.com:443?encryption=none&security=reality&sni=www.google.com&fp=chrome&pbk=KEY&sid=ab12&type=tcp&flow=xtls-rprx-vision#%D0%A2%D0%B5%D1%81%D1%82";
        let s = parse_vless(vless).unwrap();
        assert_eq!((s.address.as_str(), s.port), ("example.com", 443));
        assert_eq!(s.name, "Тест");
        assert_eq!(s.details.security.as_deref(), Some("reality"));
        assert_eq!(s.details.public_key.as_deref(), Some("KEY"));
        assert_eq!(s.details.flow.as_deref(), Some("xtls-rprx-vision"));

        let json = r#"{"v":"2","ps":"vm","add":"1.2.3.4","port":"8080","id":"uuid","aid":0,"net":"ws","path":"/p","host":"h.com","tls":"tls"}"#;
        let vmess = format!(
            "vmess://{}",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, json)
        );
        let s = parse_vmess(&vmess).unwrap();
        assert_eq!((s.name.as_str(), s.port), ("vm", 8080));
        assert_eq!(s.details.network.as_deref(), Some("ws"));
        assert_eq!(s.details.alter_id, Some(0));

        let s = parse_trojan("trojan://pa%40ss@t.example:8443?sni=t.example#T").unwrap();
        assert_eq!(s.details.id.as_deref(), Some("pa@ss"));
        assert_eq!(s.port, 8443);

        // ss в обоих формах
        let userinfo =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, "aes-256-gcm:pw");
        let s = parse_shadowsocks(&format!("ss://{userinfo}@s.example:8388#SS")).unwrap();
        assert_eq!(s.details.method.as_deref(), Some("aes-256-gcm"));
        assert_eq!((s.address.as_str(), s.port, s.name.as_str()), ("s.example", 8388, "SS"));
        let whole = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "aes-256-gcm:pw@s.example:8388",
        );
        let s = parse_shadowsocks(&format!("ss://{whole}")).unwrap();
        assert_eq!(s.details.id.as_deref(), Some("pw"));

        assert!(parse_link("http://example.com").is_err());
        assert_eq!(parse_subscription(&format!("{vless}\n\n{vmess}\nмусор")).len(), 2);
    }
}
