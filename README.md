# Claude Island

Виджет в стиле Dynamic Island для Windows 10/11: лимиты Claude, музыка, Telegram и состояние Claude Code — в маленькой капсуле у края экрана.

*A Dynamic Island–style widget for Windows showing your Claude usage limits, music, Telegram and Claude Code status. The interface is available in Russian and English.*

## Что умеет

- **Лимиты Claude** — 5-часовое окно и неделя, таймер до сброса чередуется с часами, прогноз «хватит ли до сброса».
- **Claude думает** — свечение, пока Claude отвечает в приложении Claude (Чаты и Code); зелёная вспышка, когда закончил; янтарная пульсация, когда Claude Code ждёт разрешения.
- **Музыка** — любой плеер с карточкой трека в Windows (Spotify, Яндекс Музыка, браузер): обложка, время, управление, перемотка кликом по полоске.
- **Telegram** — непрочитанные из уведомлений Telegram Desktop.
- **Система** — громкость, зарядка и батарея, смена наушников, раскладка RU/EN, камера и микрофон, «Скопировано».
- **Помодоро**, горячая клавиша (по умолчанию Win+Alt+C), скрытие в полноэкранных приложениях, автообновление.

Каждую функцию можно выключить в настройках.

## Установка

1. Скачай `ClaudeIsland_<версия>_x64-setup.exe` со страницы [Releases](../../releases/latest) и запусти. Права администратора не нужны.
   Установщик не подписан платным сертификатом, поэтому Windows может показать «Windows защитила ваш компьютер» → «Подробнее» → «Выполнить в любом случае».
2. В настройках вставь ключ сессии claude.ai: браузер → claude.ai → F12 → Application → Cookies → `sessionKey`.
   Ключ хранится только на твоём компьютере, зашифрованным Windows (DPAPI), и отправляется только на claude.ai.
3. По желанию: настройки → «Claude Code» → «Подключить» — остров начнёт показывать работу Claude Code.
4. Для Telegram: Telegram Desktop → Настройки → Уведомления → «Уведомления на рабочем столе» и «Использовать уведомления Windows».

## Сборка

Нужны [Rust](https://rustup.rs), Node.js и зависимости Tauri v2 для Windows.

```bash
npm install
npm run tauri build
```

Установщик появится в `src-tauri/target/release/bundle/nsis/`.

Выпуск версии с подписью обновлений — `scripts/release.ps1` (нужен ключ подписи, его нет в репозитории). Скрипт кладёт в `release/` установщик, `.sig` и `latest.json`; все три файла загружаются в GitHub Release с тегом `v<версия>`.

## Как устроено

- Tauri v2, интерфейс — обычные HTML/CSS/JS (`src/`), системная часть — Rust (`src-tauri/src/`).
- Лимиты: `claude.ai/api/organizations/{id}/usage` с ключом сессии пользователя.
- Музыка — Windows SMTC, Telegram — `UserNotificationListener`, «Claude думает» в чатах — UI Automation, в Claude Code — хуки, которые вызывают `claude-island.exe --hook on|off|wait`.
- Журнал ошибок: `%APPDATA%\com.iqbolovich.claudeisland\island.log`.
