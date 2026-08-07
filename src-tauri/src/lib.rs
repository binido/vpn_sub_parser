mod fetch;
mod parser;
mod ping;
mod xray;

use parser::Server;

/// Скачивает подписку и возвращает список серверов.
#[tauri::command]
async fn parse_subscription(url: String) -> Result<Vec<Server>, String> {
    if url.trim().is_empty() {
        return Err("Введите ссылку подписки".into());
    }
    let text = fetch::fetch_subscription(&url).await?;
    // Подписка бывает списком ссылок, а бывает массивом конфигов Xray.
    let servers = xray::parse_xray_json(&text).unwrap_or_else(|| parser::parse_subscription(&text));
    if servers.is_empty() {
        return Err("В ответе подписки не найдено ни одного ключа".into());
    }
    Ok(servers)
}

/// Список серверов → JSON с массивом outbounds для Xray-core.
#[tauri::command]
fn export_xray_outbounds(servers: Vec<Server>) -> String {
    xray::to_outbounds_json(&servers)
}

/// Задержки до серверов в том же порядке, что и присланный список.
/// null вместо числа — сервер не ответил.
#[tauri::command]
async fn ping_servers(servers: Vec<Server>) -> Vec<Option<u32>> {
    let targets = servers
        .iter()
        .map(|s| (s.address.clone(), s.port))
        .collect();
    ping::ping_all(targets).await
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
    url: String,
}

/// «0.2.0» новее «0.1.10»? Суффиксы вроде `-beta` игнорируются.
fn is_newer(candidate: &str, current: &str) -> bool {
    let parts = |v: &str| {
        let mut it = v
            .trim()
            .trim_start_matches('v')
            .split(['.', '-', '+'])
            .map(|p| p.parse::<u32>().unwrap_or(0));
        [it.next(), it.next(), it.next()].map(|p| p.unwrap_or(0))
    };
    parts(candidate) > parts(current)
}

/// Проверяет, нет ли в релизах репозитория версии свежее текущей.
#[tauri::command]
async fn check_update() -> Result<Option<UpdateInfo>, String> {
    let repo = env!("CARGO_PKG_REPOSITORY")
        .trim_end_matches('/')
        .strip_prefix("https://github.com/")
        .ok_or("В Cargo.toml не указан репозиторий на GitHub")?;

    let Some(release) =
        fetch::latest_release(&format!("https://api.github.com/repos/{repo}/releases/latest"))
            .await?
    else {
        return Ok(None);
    };

    let version = release["tag_name"].as_str().unwrap_or_default();
    if !is_newer(version, env!("CARGO_PKG_VERSION")) {
        return Ok(None);
    }
    Ok(Some(UpdateInfo {
        version: version.trim_start_matches('v').to_string(),
        url: release["html_url"]
            .as_str()
            .unwrap_or(env!("CARGO_PKG_REPOSITORY"))
            .to_string(),
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            parse_subscription,
            export_xray_outbounds,
            ping_servers,
            check_update
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска приложения");
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn compares_versions() {
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(is_newer("0.1.10", "0.1.9"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("v0.1.1", "0.1.1"));
        assert!(!is_newer("0.1.0", "0.1.1"));
        assert!(!is_newer("мусор", "0.1.1"));
        assert!(is_newer("0.2.0-beta1", "0.1.1"));
    }
}
