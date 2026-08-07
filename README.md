# VPN Sub Parser

Десктопное приложение на [Tauri v2](https://tauri.app): вставляешь ссылку на
VPN-подписку — получаешь читаемый список серверов с ключами, а не полотно base64.

Загрузка и разбор происходят в Rust-бэкенде, поэтому CORS и блокировки браузера
на пути не стоят, а сама подписка никуда, кроме твоей машины, не уходит.

## Что умеет

- Скачивает подписку с `User-Agent: v2rayNG/1.8.0` и понимает три формата ответа:
  готовый список ссылок, base64 (обычный и url-safe, с паддингом и без) и
  **массив конфигов Xray в JSON** — так подписку отдают панели вроде Remnawave.
- Разбирает `vless://`, `vmess://`, `trojan://`, `ss://`: uuid/пароль, адрес,
  порт, `security` (none/tls/reality), sni, fingerprint, publicKey, shortId,
  тип транспорта (tcp/ws/grpc), path, Host-заголовок и имя после `#`.
- Показывает карточки с чипами протокола, security и транспорта; по клику
  раскрывается исходная ссылка с кнопкой «Копировать».
- «Копировать все» — все ссылки построчно в буфер обмена.
- «Экспорт в Xray outbounds» — весь список в JSON-массив outbound-объектов
  формата Xray-core, с сохранением через системный диалог.
- Material You: тональная палитра от seed `#6750A4`, светлая и тёмная темы
  (по системной, с переключателем), ripple и анимации появления карточек.
- Последний URL подписки запоминается, но запрос сам по себе не выполняется.
- При запуске проверяет релизы репозитория и, если вышла версия свежее, показывает
  плашку со ссылкой. Скачивание и установку не делает — только сообщает.

Интерфейс полностью на русском, ошибки сети и парсинга показываются текстом,
а не молчаливым пустым списком.

## Сборка

### Что нужно поставить

- **Rust** 1.77.2 или новее — [rustup.rs](https://rustup.rs)
- **Node.js** 18+ — нужен только ради Tauri CLI, зависимостей времени
  выполнения у фронтенда нет
- Системные зависимости Tauri:
  - **macOS** — Xcode Command Line Tools: `xcode-select --install`
  - **Windows** — [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)
    (в Windows 11 уже есть) и Microsoft C++ Build Tools
  - **Linux** — `webkit2gtk-4.1`, `libappindicator3`, `librsvg2`, `patchelf`
    (см. [список пакетов для своего дистрибутива](https://tauri.app/start/prerequisites/))

### Запуск и сборка

```bash
npm install
npm run dev
```

```bash
npm run build
```

`npm run build` кладёт готовые артефакты в `src-tauri/target/release/bundle/`:
`.app` и `.dmg` на macOS, `.msi`/`.exe` на Windows, `.deb`/`.AppImage` на Linux.
То же самое делает `cargo tauri build`, если Tauri CLI установлен глобально.

Тесты бэкенда:

```bash
cd src-tauri && cargo test
```

## Релизы

Сборкой занимается GitHub Actions ([.github/workflows/release.yml](.github/workflows/release.yml)).
Пуш тега вида `v*` собирает четыре артефакта и складывает их в **черновик**
релиза — остаётся зайти в Releases и нажать «Publish»:

| Платформа       | Артефакт                        |
| --------------- | ------------------------------- |
| Windows x64     | `..._x64-setup.exe` (NSIS)      |
| Windows ARM64   | `..._arm64-setup.exe` (NSIS)    |
| macOS (Apple M) | `..._aarch64.dmg`               |
| Linux x64       | `..._amd64.AppImage` и `..._amd64.deb` |

Секреты не нужны — хватает штатного `GITHUB_TOKEN`.

AppImage весит под 80 МБ против 3 МБ у остальных не просто так: внутрь него
целиком уезжает движок WebKitGTK (`libwebkit2gtk` 86 МБ, `libjavascriptcoregtk`
32 МБ, `libicudata` 28 МБ в распакованном виде). На Windows и macOS вебвью даёт
система, поэтому там инсталляторы крошечные. Кому важен размер — рядом лежит
`.deb` на несколько мегабайт, он берёт `webkit2gtk-4.1` из системы.

Порядок выпуска: поднять версию **в трёх местах** —
[src-tauri/tauri.conf.json](src-tauri/tauri.conf.json),
[src-tauri/Cargo.toml](src-tauri/Cargo.toml) и [package.json](package.json)
(иначе в именах файлов останется старая версия), затем:

```bash
git commit -am "chore: release v0.2.0" && git tag v0.2.0 && git push --follow-tags
```

Сборки подписаны ad-hoc (`bundle.macOS.signingIdentity: "-"`), но не заверены
у Apple и Microsoft. Что это значит на практике:

- **macOS** — первый запуск через правый клик по приложению → «Открыть».
  Если система всё же ругается, снять карантин руками:
  `xattr -cr "/Applications/VPN Sub Parser.app"`.
- **Windows** — SmartScreen покажет «Подробнее» → «Выполнить в любом случае».

Полностью убрать предупреждения можно только платными сертификатами
(Apple Developer ID + нотаризация, code signing certificate для Windows) —
они подключаются к тому же workflow через переменные окружения Tauri.

## Структура

```
src/                        фронтенд: статические ES-модули, без бандлера
  index.html  styles.css    разметка и M3-палитра в CSS-переменных
  main.js                   склейка, темы, тосты, localStorage
  tauri.js                  обёртка над window.__TAURI__
  components/               SubscriptionInput, ServerList, ServerCard,
                            ExportButton, ripple
src-tauri/src/
  lib.rs                    команды parse_subscription, export_xray_outbounds
  fetch.rs                  HTTP-запрос и декодирование base64
  parser.rs                 парсеры четырёх протоколов и сборка ссылки обратно
  xray.rs                   серверы → outbounds Xray и разбор JSON-подписок
```

Фронтенд собран без бандлера: `withGlobalTauri` отдаёт API плагинов через
`window.__TAURI__`, поэтому npm-зависимостей во время выполнения нет, а
`npm install` нужен только чтобы получить `tauri` CLI.

## Известные особенности

- Профиль-балансировщик (в JSON-подписке — конфиг с десятками outbound'ов,
  например «Авто») схлопывается в первый сервер, как его показывают и клиенты.
- Одинаковые серверы не склеиваются: если панель отдаёт сто профилей на четыре
  адреса, в списке будет сто карточек — ровно то, что увидит VPN-клиент.
- В `Cargo.toml` включена фича `log` у `zune-jpeg` — обход бага крейта на
  rustc 1.96+; строчку можно убрать, когда исправят апстрим.
