//! Конвертация серверов в outbound-объекты Xray-core и обратно.

use crate::parser::{to_link, Details, Server};
use serde_json::{json, Map, Value};

const PROXY_PROTOCOLS: [&str; 4] = ["vless", "vmess", "trojan", "shadowsocks"];

fn str_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

fn stream_to_details(stream: Option<&Value>, details: &mut Details) {
    let Some(stream) = stream else { return };
    details.network = Some(str_field(stream, "network").unwrap_or_else(|| "tcp".into()));
    details.security = Some(str_field(stream, "security").unwrap_or_else(|| "none".into()));

    for key in ["tlsSettings", "realitySettings"] {
        if let Some(tls) = stream.get(key) {
            details.sni = str_field(tls, "serverName");
            details.fingerprint = str_field(tls, "fingerprint");
            details.public_key = str_field(tls, "publicKey");
            details.short_id = str_field(tls, "shortId");
            details.alpn = tls.get("alpn").and_then(Value::as_array).map(|list| {
                list.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(",")
            });
        }
    }

    if let Some(ws) = stream.get("wsSettings") {
        details.path = str_field(ws, "path");
        details.host = ws
            .get("headers")
            .and_then(|h| str_field(h, "Host").or_else(|| str_field(h, "host")));
    }
    if let Some(grpc) = stream.get("grpcSettings") {
        details.service_name = str_field(grpc, "serviceName");
    }
    if let Some(http) = stream.get("httpSettings") {
        details.path = str_field(http, "path");
        details.host = http
            .get("host")
            .and_then(Value::as_array)
            .and_then(|h| h.first())
            .and_then(Value::as_str)
            .map(str::to_string);
    }
}

fn outbound_to_server(outbound: &Value, name: &str) -> Option<Server> {
    let protocol = outbound.get("protocol")?.as_str()?;
    if !PROXY_PROTOCOLS.contains(&protocol) {
        return None;
    }
    let settings = outbound.get("settings")?;
    let mut details = Details::default();

    let node = settings
        .get("vnext")
        .or_else(|| settings.get("servers"))?
        .get(0)?;
    let address = str_field(node, "address")?;
    let port = node.get("port")?.as_u64()? as u16;

    match protocol {
        "vless" | "vmess" => {
            let user = node.get("users")?.get(0)?;
            details.id = str_field(user, "id");
            details.flow = str_field(user, "flow");
            details.alter_id = user.get("alterId").and_then(Value::as_u64).map(|v| v as u32);
            details.encryption = str_field(user, "encryption").or_else(|| str_field(user, "security"));
        }
        _ => {
            details.id = str_field(node, "password");
            details.method = str_field(node, "method");
        }
    }
    stream_to_details(outbound.get("streamSettings"), &mut details);

    let mut server = Server {
        protocol: protocol.to_string(),
        name: if name.is_empty() {
            address.clone()
        } else {
            name.to_string()
        },
        address,
        port,
        raw: String::new(),
        details,
    };
    server.raw = to_link(&server);
    Some(server)
}

/// Панели вроде Marzban/Remnawave отдают клиентам не список ссылок, а массив
/// готовых конфигов Xray. Возвращает None, если это не такой JSON.
pub fn parse_xray_json(text: &str) -> Option<Vec<Server>> {
    let value: Value = serde_json::from_str(text.trim()).ok()?;
    let configs: Vec<&Value> = match &value {
        Value::Array(items) => items.iter().collect(),
        Value::Object(_) => vec![&value],
        _ => return None,
    };

    let mut servers = Vec::new();
    for config in configs {
        let outbounds: Vec<&Value> = match config.get("outbounds").and_then(Value::as_array) {
            Some(list) => list.iter().collect(),
            // массив может состоять и из самих outbound-объектов
            None => vec![config],
        };
        let proxies: Vec<&Value> = outbounds
            .into_iter()
            .filter(|o| {
                o.get("protocol")
                    .and_then(Value::as_str)
                    .is_some_and(|p| PROXY_PROTOCOLS.contains(&p))
            })
            .collect();

        match str_field(config, "remarks") {
            // ponytail: профиль с именем = одна карточка. Профиль-балансировщик
            // из десятков outbound'ов схлопывается в первый — так же, как его
            // показывают клиенты.
            Some(remarks) => {
                if let Some(server) = proxies.first().and_then(|o| outbound_to_server(o, &remarks)) {
                    servers.push(server);
                }
            }
            None => servers.extend(proxies.into_iter().filter_map(|o| {
                outbound_to_server(o, o.get("tag").and_then(Value::as_str).unwrap_or(""))
            })),
        }
    }

    (!servers.is_empty()).then_some(servers)
}

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

    #[test]
    fn parses_subscription_of_xray_configs() {
        let text = r#"[{"remarks":"🇳🇱 Нидерланды","outbounds":[
            {"tag":"proxy","protocol":"vless","settings":{"vnext":[{"address":"nl.example.com","port":443,
              "users":[{"id":"uuid-1","encryption":"none","flow":"xtls-rprx-vision"}]}]},
             "streamSettings":{"network":"tcp","security":"reality",
              "realitySettings":{"serverName":"nl.example.com","publicKey":"PBK","shortId":"ab12","fingerprint":"firefox"}}},
            {"tag":"direct","protocol":"freedom"}]}]"#;

        let servers = parse_xray_json(text).unwrap();
        assert_eq!(servers.len(), 1);
        let s = &servers[0];
        assert_eq!((s.name.as_str(), s.address.as_str(), s.port), ("🇳🇱 Нидерланды", "nl.example.com", 443));
        assert_eq!(s.details.public_key.as_deref(), Some("PBK"));

        // собранная ссылка должна разбираться обратно в тот же сервер
        let back = crate::parser::parse_link(&s.raw).unwrap();
        assert_eq!(back.name, s.name);
        assert_eq!(back.address, s.address);
        assert_eq!(back.details.short_id, s.details.short_id);
        assert_eq!(back.details.flow, s.details.flow);
        assert_eq!(to_outbound(&back)["streamSettings"], to_outbound(s)["streamSettings"]);

        assert!(parse_xray_json("не json").is_none());
        assert!(parse_xray_json(r#"{"outbounds":[{"protocol":"freedom"}]}"#).is_none());
    }
}
