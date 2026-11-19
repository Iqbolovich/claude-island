// Перевод интерфейса. Ключ — русская строка, {0} {1} — подстановки.
// Язык: настройка lang (auto | ru | en); auto — по языку Windows.
(function () {
  const EN = {
    // остров
    'нет связи': 'offline', 'нет данных': 'no data', 'Нет данных': 'No data',
    'обновлено только что': 'updated just now', 'обновлено {0} мин назад': 'updated {0} min ago',
    'нет связи · {0} мин назад': 'offline · {0} min ago',
    'Claude · лимиты': 'Claude · limits', '5-часовое окно': '5-hour window', 'Неделя': 'Week',
    'сброс через': 'resets in', 'сброс {0}': 'resets {0}', 'сброс —': 'resets —',
    'Нет связи с Claude': 'No connection to Claude',
    '⛔ Лимит исчерпан до сброса': '⛔ Limit used up until reset',
    '✓ При текущем темпе хватит до сброса': '✓ At this pace it lasts until reset',
    '⚠ При текущем темпе лимит кончится через ~{0}': '⚠ At this pace the limit runs out in ~{0}',
    'Ничего не играет': 'Nothing playing', 'Включи трек в любом плеере': 'Play a track in any player',
    'Telegram · {0} непрочит. · клик, чтобы прочитать': 'Telegram · {0} unread · click to mark read',
    'Нет доступа к уведомлениям — разреши в Параметрах Windows': 'No access to notifications — allow it in Windows Settings',
    'Нет новых сообщений': 'No new messages', '{0} и ещё {1}': '{0} and {1} more',
    'Claude ждёт разрешения': 'Claude needs permission', 'Claude закончил': 'Claude is done',
    'Скопировано': 'Copied', 'Батарея {0}% — подключи зарядку': 'Battery {0}% — plug in the charger',
    'Зарядка · {0}%': 'Charging · {0}%', 'От батареи · {0}%': 'On battery · {0}%',
    'Камера · {0}': 'Camera · {0}', 'Микрофон · {0}': 'Microphone · {0}',
    'Помодоро · {0} мин': 'Pomodoro · {0} min', 'Время вышло — сделай перерыв': "Time's up — take a break",
    'Таймер остановлен': 'Timer stopped', 'Доступно обновление {0}': 'Update {0} available',
    'Настройки': 'Settings', 'Таймер': 'Timer', 'Скрыть остров': 'Hide the island',
    'Предыдущий трек': 'Previous track', 'Играть или пауза': 'Play or pause', 'Следующий трек': 'Next track',
    // ошибки из Rust
    'Ключ устарел — обнови его в настройках': 'Key expired — update it in settings',
    'Нет связи с claude.ai': 'No connection to claude.ai', 'Непонятный ответ claude.ai': 'Unexpected reply from claude.ai',
    'Вставь ключ сессии в настройках': 'Paste your session key in settings',
    'В аккаунте не нашлось лимитов подписки': 'No subscription limits found in this account',
    // настройки
    'Claude Island — настройки': 'Claude Island — settings',
    'Лимиты Claude': 'Claude limits', 'Сохранить': 'Save', 'Ключ не задан': 'No key set',
    'Где взять ключ сессии': 'Where to get the session key',
    'Открой claude.ai в браузере и войди.': 'Open claude.ai in a browser and sign in.',
    'F12 → Application → Cookies → https://claude.ai.': 'F12 → Application → Cookies → https://claude.ai.',
    'Скопируй значение sessionKey и вставь сюда.': 'Copy the sessionKey value and paste it here.',
    'Ключ хранится только на этом компьютере, зашифрованным средствами Windows.': 'The key stays on this computer only, encrypted by Windows.',
    'Обновлять каждые, сек': 'Refresh every, sec', 'Жёлтый порог, %': 'Yellow threshold, %', 'Красный порог, %': 'Red threshold, %',
    'Остров': 'Island', 'Положение': 'Position', 'Сверху слева': 'Top left', 'Сверху по центру': 'Top center',
    'Сверху справа': 'Top right', 'Снизу слева': 'Bottom left', 'Снизу по центру': 'Bottom center', 'Снизу справа': 'Bottom right',
    'Тема': 'Theme', 'Чёрная': 'Black', 'Стекло': 'Glass', 'Прозрачность стекла': 'Glass opacity',
    'Таймер ↔ часы, сек': 'Timer ↔ clock, sec', 'Музыка': 'Music', 'Язык': 'Language', 'Как в Windows': 'Same as Windows',
    'Прятать в полноэкранных приложениях': 'Hide in full-screen apps',
    'Показывать на острове': 'Show on the island',
    'Громкость': 'Volume', 'Свечение при play/pause': 'Glow on play/pause', 'Скопировано в буфер': 'Copied to clipboard', 'Зарядка и батарея': 'Charging and battery',
    'Смена наушников и колонок': 'Headphones and speakers switch', 'Claude закончил ответ': 'Claude finished replying',
    'Claude ждёт разрешения (Code)': 'Claude needs permission (Code)', 'Раскладка RU/EN': 'Keyboard layout',
    'Камера и микрофон': 'Camera and microphone', 'Помодоро, мин': 'Pomodoro, min',
    'Claude Code': 'Claude Code', 'Подключить': 'Connect', 'Отключить': 'Disconnect',
    'Подключено': 'Connected', 'Не подключено': 'Not connected',
    'Остров покажет, когда Claude Code думает, закончил или ждёт разрешения. Работает в новых сессиях.':
      'The island shows when Claude Code is thinking, done, or waiting for permission. Works in new sessions.',
    'Система': 'System', 'Запускать вместе с Windows': 'Start with Windows',
    'Показать/спрятать остров': 'Show/hide the island', 'Нажми сочетание…': 'Press keys…', 'нет': 'none',
    'Сочетание занято другой программой': 'This shortcut is taken by another app',
    'Проверять обновления': 'Check for updates', 'Проверить': 'Check', 'Обновить до {0}': 'Update to {0}',
    'Версия {0} — последняя': 'Version {0} is the latest', 'Не удалось проверить': 'Could not check',
    'Скачиваю…': 'Downloading…', 'Журнал ошибок': 'Error log', 'Открыть': 'Open',
    '✓ Сохранено': '✓ Saved', 'Применить': 'Apply', 'Ключ сохранён и зашифрован': 'Key saved and encrypted',
    'Проверяю…': 'Checking…', 'Удаляю…': 'Removing…', 'Ключ удалён': 'Key removed',
    '✓ Работает: 5 ч — {0}%, неделя — {1}%': '✓ Works: 5 h — {0}%, week — {1}%',
    'Ключ сохранён, но: {0}': 'Key saved, but: {0}', 'Ошибка: {0}': 'Error: {0}', 'Версия {0}': 'Version {0}',
  };
  let lang = 'ru';
  const fill = (s, a) => s.replace(/\{(\d)\}/g, (_, i) => a[i] ?? '');
  window.t = (ru, ...a) => {
    let s = ru;
    if (lang === 'en') {
      if (EN[ru]) s = EN[ru];
      else if (ru.startsWith('claude.ai отказал')) s = ru.replace('claude.ai отказал', 'claude.ai refused');
    }
    return fill(s, a);
  };
  window.setLang = pref => {
    lang = pref === 'ru' || pref === 'en' ? pref : ((navigator.language || 'ru').toLowerCase().startsWith('ru') ? 'ru' : 'en');
    document.documentElement.lang = lang;
    // элементы с data-t: оригинал (русский) хранится в атрибуте, переводим заново при каждой смене
    document.querySelectorAll('[data-t]').forEach(el => {
      if (!el.dataset.t) el.dataset.t = el.textContent.trim();
      el.textContent = t(el.dataset.t);
    });
    document.querySelectorAll('[data-tt]').forEach(el => { el.title = t(el.dataset.tt); el.setAttribute('aria-label', t(el.dataset.tt)); });
    if (document.title) { document.title = t(document.documentElement.dataset.title || (document.documentElement.dataset.title = document.title)); }
    return lang;
  };
  window.getLang = () => lang;
  window.locale = () => (lang === 'en' ? 'en-US' : 'ru-RU');
})();
