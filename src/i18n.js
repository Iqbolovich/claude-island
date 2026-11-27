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
    'Нет новых сообщений': 'No new messages', 'Новых сообщений: {0} — открой Telegram, чтобы прочитать': 'New messages: {0} — open Telegram to read', 'Новых сообщений: {0}': 'New messages: {0}', '{0} и ещё {1}': '{0} and {1} more',
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
    'Обзор': 'Overview',
    'Что показывать': 'What to show',
    'Что сейчас работает. Нажми на плитку, чтобы настроить.': 'What is working now. Click a tile to set it up.',
    'Обновления': 'Updates',
    'Меняется сразу, остров перекрасится на глазах.': 'Applies instantly — watch the island change.',
    'Где стоит остров': 'Island position',
    'Нажми на место на схеме экрана.': 'Click a spot on the screen diagram.',
    'Остров показывает, сколько осталось до лимита и когда сброс.': 'The island shows how much is left and when it resets.',
    'Вход': 'Sign-in',
    'Откроется окно claude.ai — войди как обычно, ключ сохранится сам.': 'A claude.ai window opens — sign in as usual, the key is saved automatically.',
    'Вставить ключ вручную': 'Paste the key manually',
    'Предупреждения': 'Warnings',
    'Остров подсветится жёлтым': 'The island glows yellow',
    'Остров мигнёт красным': 'The island flashes red',
    'Новые сообщения из Telegram Desktop — прямо на острове.': 'New Telegram Desktop messages, right on the island.',
    'Показывать сообщения Telegram': 'Show Telegram messages',
    'Проверка': 'Check',
    'Выключи то, что отвлекает.': 'Turn off whatever distracts you.',
    'Поведение': 'Behaviour',
    'Запуск, горячая клавиша и обновления.': 'Startup, hotkey and updates.',
    'Если что-то не так': 'If something is wrong',
    'Версия {0} · проверяются сами': 'Version {0} · checked automatically',
    'Версия {0} · автопроверка выключена': 'Version {0} · auto-check off',
    'Вход выполнен': 'Signed in',
    'Нужно войти в claude.ai': 'Sign in to claude.ai',
    'Работает: 5 ч — {0}%, неделя — {1}%': 'Working: 5 h — {0}%, week — {1}%',
    'Выключено': 'Off',
    'Нет доступа к уведомлениям': 'No notification access',
    'Работает': 'Working',
    'Работает по счётчику в заголовке': 'Working via the window title count',
    'Тема': 'Theme', 'Оформление': 'Appearance', 'Чёрная': 'Black', 'Стекло': 'Glass', 'Светлое стекло': 'Light glass', 'Тёплая бумага': 'Warm paper', 'Терминал': 'Terminal', 'Закат': 'Sunset', 'Прозрачность': 'Opacity',
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
    'Скачиваю…': 'Downloading…', 'Журнал ошибок': 'Error log',
    'Добро пожаловать!': 'Welcome!', 'Три шага — и остров готов.': 'Three steps and the island is ready.',
    'Войти в claude.ai — остров покажет лимиты': 'Sign in to claude.ai to see your limits',
    'Подключить Claude Code (если пользуешься)': 'Connect Claude Code (if you use it)',
    'Telegram: проверить уведомления': 'Telegram: check notifications', 'Войти': 'Sign in', 'Войти заново': 'Sign in again',
    'Готово': 'Done', 'Вход в claude.ai': 'Sign in to claude.ai',
    'Откроется окно claude.ai — войди как обычно, ключ сохранится сам. Или вставь его вручную:': 'A claude.ai window opens: sign in as usual and the key is saved automatically. Or paste it manually:',
    'Войди в открывшемся окне — ключ сохранится сам': 'Sign in in the window that opened; the key will be saved automatically',
    'Уведомления Windows': 'Windows notifications', 'Если сообщения Telegram не появляются': "If Telegram messages don't show up",
    'Telegram Desktop → Настройки → Уведомления и звуки: включи «Уведомления на рабочем столе» и «Использовать уведомления Windows».': 'Telegram Desktop → Settings → Notifications and Sounds: turn on "Desktop notifications" and "Use Windows notifications".',
    'Там же включи «Показывать текст сообщения», иначе в уведомлении будет только «Новое сообщение».': 'Also turn on "Show message preview", otherwise a notification only says "New message".',
    'Параметры Windows → Система → Уведомления: уведомления включены, Telegram в списке разрешён, режим «Не беспокоить» выключен.': 'Windows Settings → System → Notifications: notifications on, Telegram allowed, Do not disturb off.',
    'Параметры Windows → Конфиденциальность → Уведомления: доступ приложений к уведомлениям разрешён.': 'Windows Settings → Privacy → Notifications: apps may access notifications.',
    'Даже без уведомлений остров покажет число непрочитанных — его видно в заголовке окна Telegram.': 'Even without notifications the island shows the unread count, taken from the Telegram window title.',
    'Доступ к уведомлениям Windows разрешён': 'Access to Windows notifications allowed',
    'Нет доступа к уведомлениям Windows — разреши в Параметрах': 'No access to Windows notifications — allow it in Settings',
    'Telegram Desktop запущен': 'Telegram Desktop is running', 'Telegram Desktop не запущен': 'Telegram Desktop is not running',
    'Уведомления Telegram приходят в Windows': 'Telegram notifications reach Windows',
    'Уведомлений Telegram в Windows пока не было — проверь настройки ниже': 'No Telegram notifications in Windows yet — check the settings below',
    'Непрочитанных по заголовку окна: {0}': 'Unread by window title: {0}', 'Открыть': 'Open',
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
