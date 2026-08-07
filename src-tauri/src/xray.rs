//! Конвертация серверов в outbound-объекты Xray-core.

use crate::parser::Server;
use serde_json::{json, Map, Value};

fn insert_some(map: &mut Map<String, Value>, key: &str, value: Option<&String>) {
    if let Some(v) = value {
        map.insert(key.into(), json!(v));
    }
}

fn stream_settings(s: &Server) -> Value {
    let d = &s.details;
    let network = d.network.clone().unwrap_or_else(|| "tcp".into());
    let security = match d.security.as_deref() {
        Some("") | None => "none".to_string(),
        Some(other) => other.to_string(),
    };

    let mut stream = Map::new();
    stream.insert("network".into(), json!(network));
    stream.insert("security".into(), json!(security));

    let server_name = d.sni.clone().unwrap_or_else(|| s.address.clone());
    let alpn: Option<Vec<&str>> = d
        .alpn
        .as_ref()
        .map(|a| a.split(',').map(str::trim).filter(|p| !p.is_empty()).collect());

    match security.as_str() {
        "tls" => {
            let mut tls = Map::new();
            tls.insert("serverName".into(), json!(server_name));
            insert_some(&mut tls, "fingerprint", d.fingerprint.as_ref());
            if let Some(alpn) = alpn {
                tls.insert("alpn".into(), json!(alpn));
            }
            stream.insert("tlsSettings".into(), Value::Object(tls));
        }
        "reality" => {
            let mut reality = Map::new();
            reality.insert("serverName".into(), json!(server_name));
            insert_some(&mut reality, "fingerprint", d.fingerprint.as_ref());
            insert_some(&mut reality, "publicKey", d.public_key.as_ref());
            insert_some(&mut reality, "shortId", d.short_id.as_ref());
            stream.insert("realitySettings".into(), Value::Object(reality));
        }
        _ => {}
    }

    match network.as_str() {
        "ws" => {
            let mut ws = Map::new();
            ws.insert(
                "path".into(),
                json!(d.path.clone().unwrap_or_else(|| "/".into())),
            );
            if let Some(host) = &d.host {
                ws.insert("headers".into(), json!({ "Host": host }));
            }
            stream.insert("wsSettings".into(), Value::Object(ws));
        }
        "grpc" => {
            let service = d
                .service_name
                .clone()
                .or_else(|| d.path.clone())
                .unwrap_or_default();
            stream.insert("grpcSettings".into(), json!({ "serviceName": service }));
        }
        "http" | "h2" => {
            let mut http = Map::new();
            http.insert(
                "path".into(),
                json!(d.path.clone().unwrap_or_else(|| "/".into())),
            );
            if let Some(host) = &d.host {
                http.insert("host".into(), json!([host]));
            }
            stream.insert("httpSettings".into(), Value::Object(http));
        }
        _ => {}
    }

    Value::Object(stream)
}

pub fn to_outbound(s: &Server) -> Value {
    let d = &s.details;
    let id = d.id.clone().unwrap_or_default();

    let settings = match s.protocol.as_str() {
        "vless" => {
            let mut user = Map::new();
            user.insert("id".into(), json!(id));
            user.insert(
                "encryption".into(),
                json!(d.encryption.clone().unwrap_or_else(|| "none".into())),
            );
            insert_some(&mut user, "flow", d.flow.as_ref());
            json!({ "vnext": [{ "address": s.address, "port": s.port, "users": [Value::Object(user)] }] })
        }
        "vmess" => json!({ "vnext": [{
            "address": s.address,
            "port": s.port,
            "users": [{
                "id": id,
                "alterId": d.alter_id.unwrap_or(0),
                "security": d.encryption.clone().unwrap_or_else(|| "auto".into()),
            }],
        }] }),
        "trojan" => json!({ "servers": [{ "address": s.address, "port": s.port, "password": id }] }),
        _ => json!({ "servers": [{
            "address": s.address,
            "port": s.port,
            "method": d.method.clone().unwrap_or_else(|| "aes-256-gcm".into()),
            "password": id,
        }] }),
    };

    json!({
        "tag": s.name,
        "protocol": s.protocol,
        "settings": settings,
        "streamSettings": stream_settings(s),
    })
}

pub fn to_outbounds_json(servers: &[Server]) -> String {
    let outbounds: Vec<Value> = servers.iter().map(to_outbound).collect();
    serde_json::to_string_pretty(&outbounds).unwrap_or_else(|_| "[]".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_link;

    #[test]
    fn builds_reality_outbound() {
        let s = parse_link("vless://uuid@example.com:443?security=reality&pbk=KEY&sid=ab&fp=chrome&flow=xtls-rprx-vision&type=tcp#Node").unwrap();
        let o = to_outbound(&s);
        assert_eq!(o["protocol"], "vless");
        assert_eq!(o["tag"], "Node");
        assert_eq!(o["settings"]["vnext"][0]["users"][0]["flow"], "xtls-rprx-vision");
        assert_eq!(o["streamSettings"]["realitySettings"]["publicKey"], "KEY");
        assert_eq!(o["streamSettings"]["realitySettings"]["serverName"], "example.com");

        let ws = parse_link("trojan://pw@t.example:443?type=ws&path=/x&host=h.com&security=tls#T").unwrap();
        let o = to_outbound(&ws);
        assert_eq!(o["settings"]["servers"][0]["password"], "pw");
        assert_eq!(o["streamSettings"]["wsSettings"]["headers"]["Host"], "h.com");
        assert!(to_outbounds_json(&[s, ws]).starts_with('['));
    }
}
