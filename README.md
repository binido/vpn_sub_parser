# VPN Sub Parser

Десктопное приложение на [Tauri v2](https://tauri.app). Принимает ссылку на
VPN-подписку, скачивает и разбирает её и показывает список серверов: протокол,
адрес, порт, параметры транспорта и исходную ссылку.

Понимает три формата ответа подписки — список ссылок, base64 и массив конфигов
Xray в JSON. Разбирает `vless://`, `vmess://`, `trojan://`, `ss://`. Умеет
измерять задержку до серверов, фильтровать и сортировать список, копировать
ссылки и выгружать всё в JSON с outbound-объектами Xray-core.

Загрузка и разбор выполняются в Rust-бэкенде.

## Требования

- **Rust** 1.77.2 или новее — [rustup.rs](https://rustup.rs)
- **Node.js** 18+ — только для Tauri CLI, зависимостей времени выполнения
  у фронтенда нет
- Системные зависимости Tauri:
  - **macOS** — Xcode Command Line Tools: `xcode-select --install`
  - **Windows** — [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)
    (в Windows 11 предустановлен) и Microsoft C++ Build Tools
  - **Linux** — `webkit2gtk-4.1`, `libappindicator3`, `librsvg2`, `patchelf`
    ([пакеты по дистрибутивам](https://tauri.app/start/prerequisites/))

## Запуск

```bash
npm install
```

```bash
npm run dev
```

## Сборка

```bash
npm run build
```

Артефакты появляются в `src-tauri/target/release/bundle/`: `.app` и `.dmg` на
macOS, `.exe` на Windows, `.AppImage` и `.deb` на Linux.

## Тесты

```bash
cd src-tauri && cargo test
```
