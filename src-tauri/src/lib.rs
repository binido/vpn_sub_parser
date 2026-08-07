mod fetch;
mod parser;
mod xray;

use parser::Server;

/// Скачивает подписку и возвращает список серверов.
#[tauri::command]
async fn parse_subscription(url: String) -> Result<Vec<Server>, String> {
    if url.trim().is_empty() {
        return Err("Введите ссылку подписки".into());
    }
    let text = fetch::fetch_subscription(&url).await?;
    let servers = parser::parse_subscription(&text);
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            parse_subscription,
            export_xray_outbounds
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска приложения");
}
