// Claude Island — логика острова.
// Данные приходят из Rust событиями (usage-update, media-update, tg-notification).
// Без Tauri (обычный браузер) включается демо-режим, чтобы можно было смотреть визуал.

const $ = id => document.getElementById(id);
const isl = $('island'), stage = $('stage');
const TAURI = window.__TAURI__;
const invoke = TAURI ? TAURI.core.invoke : async () => {};
const listen = TAURI ? TAURI.event.listen : async () => {};
const RM = matchMedia('(prefers-reduced-motion: reduce)').matches;
const W5 = 5 * 3600e3;                       // окно 5 часов
const STALE = 5 * 60e3;                      // данные старше 5 мин = нет связи

const S = {
  pct: 0, week: 0, resetAt: 0, weekResetAt: 0, lastUpdate: 0, ok: false,
  media: { has: false, playing: false, title: '', artist: '', pos: 0, dur: 0, cover: '', accent: '' },
  unread: [], pinned: false, hidden: false, thinking: false, warned: {}, hist: [], flash: 0
};

try { S.hist = JSON.parse(localStorage.getItem('ci-hist') || '[]'); } catch {}

// пороги и прочее приходят из настроек (settings-update)
const CFG = { warn: 80, alarm: 95, swap: 4, tg: true, fx: {}, pomo: 25 };
const on = k => CFG.fx[k] !== false;
const color = p => p >= CFG.alarm ? 'var(--bad)' : p >= CFG.warn ? 'var(--warn)' : 'var(--ok)';
const fmt = ms => { ms = Math.max(0, ms); const s = Math.floor(ms / 1e3), h = Math.floor(s / 3600), m = Math.floor(s % 3600 / 60), x = s % 60;
  return `${h}:${String(m).padStart(2, '0')}:${String(x).padStart(2, '0')}`; };
const hm = ms => fmt(ms).slice(0, -3);       // без секунд
const mmss = s => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`;

/* ---------- размер острова и область клика ---------- */
function fit() {
  // высоту берём по последнему ВИДИМОМУ блоку: скрытые плитки (Telegram без сообщений) не в счёт
  const open = isl.classList.contains('open');
  const l = [...$('vFull').children].reverse().find(x => x.offsetHeight > 0) || $('vFull').lastElementChild;
  isl.style.height = open && !S.hidden ? (l.offsetTop + l.offsetHeight + 18 + 2) + 'px' : '';
  const chip = isl.classList.contains('chip') && !S.hidden;
  const compact = !open && !S.hidden && !isl.classList.contains('notif') && !chip;
  const [a, b] = $('vCompact').children;
  isl.style.width = compact ? (a.offsetWidth + b.offsetWidth + 14 + 24 + 2) + 'px'
    : chip ? ($('cText').offsetWidth + 22 + 10 + 32 + 2) + 'px' : '';
}
new ResizeObserver(fit).observe($('vFull'));

// Окно приложения равно капсуле: так Windows может размыть под ней рабочий стол (Acrylic),
// а клики снаружи проходят насквозь сами собой. Пружину рисует CSS, окно за ней следует.
if (TAURI) document.documentElement.classList.add('tauri');
let lastSize = '', rafUntil = 0, rafOn = false, sizeBusy = false, sizeNext = null;
// не больше одного запроса в полёте: пока окно применяет размер, копим только самый свежий
function sendSize(sz) {
  if (sizeBusy) { sizeNext = sz; return; }
  sizeBusy = true;
  invoke('set_island_size', sz).catch(() => {}).finally(() => {
    sizeBusy = false;
    if (sizeNext) { const n = sizeNext; sizeNext = null; sendSize(n); }
  });
}
function syncSize() {
  // окно не сжимается вслед за нажатием (scale < 1) — иначе оно дёргается; «толчок» вверх помещается
  const b = isl.getBoundingClientRect();
  const w = Math.round(Math.max(b.width, isl.offsetWidth)), h = Math.round(Math.max(b.height, isl.offsetHeight));
  const key = w + ',' + h;
  if (key !== lastSize) { lastSize = key; sendSize({ w, h }); }
}
// Раньше размер проверялся 60 раз в секунду всегда — renderer и GPU не засыпали никогда.
// Теперь цикл крутится только пока капсула меняет размер или «толкается», потом останавливается.
function kick(ms = 800) {
  if (!TAURI) return;
  rafUntil = Math.max(rafUntil, performance.now() + ms);
  if (!rafOn) { rafOn = true; requestAnimationFrame(sizeLoop); }
}
function sizeLoop(t) {
  syncSize();
  if (t < rafUntil) requestAnimationFrame(sizeLoop); else rafOn = false;
}
if (TAURI) {
  new ResizeObserver(() => kick(700)).observe(isl);
  isl.addEventListener('transitionrun', e => { if (e.target === isl) kick(900); });
  isl.addEventListener('animationstart', e => { if (e.target === isl && !e.pseudoElement) kick(1200); });
  for (const ev of ['mousedown', 'mouseup']) isl.addEventListener(ev, () => kick(500));
  kick(1500);
}

/* ---------- виды ---------- */
function setView(v) {
  if (S.hidden) return;
  ['vCompact', 'vFull', 'vNotif', 'vChip'].forEach(id => $(id).classList.toggle('on', id === v));
  isl.classList.toggle('open', v === 'vFull');
  isl.classList.toggle('notif', v === 'vNotif');
  isl.classList.toggle('chip', v === 'vChip');
  fit();
}
[...$('vFull').children].forEach((c, i) => c.style.setProperty('--i', i));

function fx(cls, ms) { isl.classList.remove(cls); void isl.offsetWidth; isl.classList.add(cls); setTimeout(() => isl.classList.remove(cls), ms); }
function pop(el, from = .6) { if (!RM) el.animate([{ transform: `scale(${from})`, opacity: .3 }, { transform: 'scale(1.15)', opacity: 1, offset: .55 }, { transform: 'scale(1)' }], { duration: 420, easing: 'ease-out' }); }

let hover = false, popT, hoverT;
function popFull(ms, flash) {
  if (S.hidden && S.pct < CFG.alarm) return;
  if (S.hidden) unhide();
  if (flash) { S.flash = 1; $('forecast').textContent = flash; setTimeout(() => { S.flash = 0; }, ms); }
  setView('vFull'); clearTimeout(popT);
  popT = setTimeout(() => { if (!hover && !S.pinned) setView('vCompact'); }, ms);
}
function toggle() { if (S.hidden) return unhide(); S.pinned = !isl.classList.contains('open'); setView(S.pinned ? 'vFull' : 'vCompact'); }
function hide() { S.hidden = true; S.pinned = false; isl.classList.remove('open', 'notif'); isl.classList.add('hidden'); fit(); }
function unhide() { S.hidden = false; isl.classList.remove('hidden'); setView('vCompact'); }

/* ---------- плавный счётчик процентов ---------- */
let shown = 0, tw;
function tweenTo(p) {
  cancelAnimationFrame(tw);
  const a = shown, t0 = performance.now(), d = RM ? 0 : 600;
  const step = t => {
    const k = Math.min(1, (t - t0) / (d || 1)), e = 1 - Math.pow(1 - k, 3);
    shown = a + (p - a) * e;
    const v = Math.round(shown) + '%';
    $('fPct').textContent = v;
    if (!isl.classList.contains('stale')) $('cPct').textContent = v;
    if (k < 1) tw = requestAnimationFrame(step);
  };
  tw = requestAnimationFrame(step);
}

/* ---------- спарклайн расхода ---------- */
function spark() {
  if (!S.hist.length || !S.resetAt) return;
  const key = S.hist.length + ':' + S.pct + ':' + S.resetAt;
  if (key === spark.k) return; spark.k = key;
  const start = S.resetAt - W5, x = t => ((t - start) / W5 * 300).toFixed(1), y = p => (28 - p * .26).toFixed(1);
  const pts = S.hist.map(([t, p]) => `${x(t)},${y(p)}`).join(' ');
  const last = S.hist.at(-1), c = color(S.pct);
  $('spark').innerHTML = `<line x1="0" y1="${y(CFG.warn)}" x2="300" y2="${y(CFG.warn)}" stroke="rgba(255,255,255,.18)" stroke-dasharray="3 3"/>
   <polygon class="ar" points="${x(start)},28 ${pts} ${x(last[0])},28" fill="${c}" opacity=".15"/>
   <polyline class="ln" pathLength="1" points="${pts}" fill="none" stroke="${c}" stroke-width="1.5" vector-effect="non-scaling-stroke"/>
   <circle cx="${x(last[0])}" cy="${y(last[1])}" r="2.5" fill="${c}"/>`;
}

/* ---------- отрисовка ---------- */
function render() {
  const now = Date.now();
  const stale = !S.ok || !S.lastUpdate || now - S.lastUpdate > STALE;
  const left = S.resetAt ? S.resetAt - now : 0;
  const c = stale ? 'var(--stale)' : color(S.pct);
  isl.classList.toggle('stale', stale);
  isl.classList.toggle('thinking', S.thinking && !S.waiting);
  isl.classList.toggle('waiting', !!S.waiting);

  $('cPct').textContent = stale ? '—' : Math.round(shown) + '%';
  $('fPct').textContent = stale ? '—' : Math.round(shown) + '%';
  $('cTimer').textContent = compactText(left, stale);
  $('fTimer').textContent = stale ? '—' : hm(left);
  $('ringFg').style.strokeDashoffset = 56.5 * (1 - (stale ? 0 : S.pct) / 100);
  $('ringFg').style.stroke = c;
  $('b5').style.width = S.pct + '%'; $('b5').style.background = c;
  $('wPct').textContent = stale ? '—' : S.week + '%';
  $('bw').style.width = S.week + '%'; $('bw').style.background = stale ? c : color(S.week);
  $('wReset').textContent = S.weekResetAt
    ? t('сброс {0}', new Date(S.weekResetAt).toLocaleString(locale(), { weekday: 'short', hour: '2-digit', minute: '2-digit' }))
    : t('сброс —');

  const ago = Math.floor((now - S.lastUpdate) / 60e3);
  $('upd').textContent = !S.lastUpdate ? t('нет данных') : stale ? t('нет связи · {0} мин назад', ago) : ago < 1 ? t('обновлено только что') : t('обновлено {0} мин назад', ago);
  $('upd').classList.toggle('bad', stale);

  // прогноз: темп = % за прошедшую часть окна
  if (!S.flash) {
    if (stale) $('forecast').textContent = S.err ? t(S.err) : t('Нет связи с Claude');
    else {
      const elapsed = W5 - left, rate = S.pct / Math.max(elapsed, 60e3), until = (100 - S.pct) / rate;
      $('forecast').textContent = S.pct >= 100 ? t('⛔ Лимит исчерпан до сброса')
        : until > left ? t('✓ При текущем темпе хватит до сброса')
        : t('⚠ При текущем темпе лимит кончится через ~{0}', hm(until));
    }
  }
  spark();

  // музыка
  const m = S.media;
  // SMTC отдаёт позицию событиями и между ними она стоит, поэтому догоняем её локально
  const pos = m.dur ? Math.min(m.dur, m.pos + (m.playing ? (now - (m.posAt || now)) / 1000 : 0)) : m.pos;
  isl.classList.toggle('playing', m.playing);
  isl.classList.toggle('hasMedia', m.has);
  $('song').textContent = m.has ? m.title : t('Ничего не играет');
  $('artist').textContent = m.has ? (m.dur ? `${m.artist} · ${mmss(pos)} / ${mmss(m.dur)}` : m.artist) : t('Включи трек в любом плеере');
  $('prog').style.width = (m.dur ? pos / m.dur * 100 : 0) + '%';
  $('mPlay').textContent = m.playing ? '⏸' : '▶';
  ['mPrev', 'mPlay', 'mNext'].forEach(b => { $(b).disabled = !m.has; });

  // telegram
  const n = S.unread.length;
  isl.classList.toggle('hasUnread', n > 0);
  // плитка только когда есть что показать (или нужно попросить доступ)
  const tgHide = !CFG.tg || (n === 0 && S.tg !== 'denied');
  if ($('tTg').hidden !== tgHide) { $('tTg').hidden = tgHide; fit(); }
  $('dot').textContent = n;
  $('tgCount').textContent = n ? t('Telegram · {0} непрочит. · клик, чтобы прочитать', n) : 'Telegram';
  $('msgs').innerHTML = n
    ? S.unread.slice(-3).reverse().map(x => `<div><b>${esc(x.from)}:</b> ${esc(x.text)}</div>`).join('')
    : `<div>${S.tg === 'denied' ? t('Нет доступа к уведомлениям — разреши в Параметрах Windows') : t('Нет новых сообщений')}</div>`;

  fit();
}
const esc = s => String(s).replace(/[&<>"]/g, ch => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[ch]));
setInterval(render, 1000);

/* ---------- чередование таймер ↔ часы ---------- */
let clockMode = false;
// часы показываем всегда, даже без связи: чередуются с таймером или с «нет связи»
function compactText(left, stale = isl.classList.contains('stale')) {
  if (S.volUntil > Date.now()) return S.volText;
  if (S.pomoEnd) return '🍅 ' + mmss(Math.max(0, (S.pomoEnd - Date.now()) / 1000));
  if (clockMode) return new Date().toLocaleTimeString(locale(), { hour: '2-digit', minute: '2-digit' });
  return stale ? t('нет связи') : '↻ ' + hm(left);
}
// ширина фиксируется по самому длинному варианту, чтобы остров не дышал при смене
function lockWidth() {
  const el = $('cTimer'), txt = el.textContent; el.style.minWidth = ''; let w = 0;
  for (const s of ['↻ 0:00', '00:00', t('нет связи'), '🍅 00:00', '🔊 100%']) { el.textContent = s; w = Math.max(w, el.offsetWidth); }
  el.style.minWidth = w + 'px'; el.textContent = txt; fit();
}
document.fonts && document.fonts.ready.then(lockWidth);
let swapTick = 0;
setInterval(() => {
  if (++swapTick < CFG.swap) return; swapTick = 0;
  const el = $('cTimer');
  const swap = () => { clockMode = !clockMode; el.textContent = compactText(S.resetAt - Date.now()); fit(); };
  if (RM) return swap();
  el.animate([{ transform: 'none', opacity: 1, filter: 'none' }, { transform: 'translateY(-8px)', opacity: 0, filter: 'blur(3px)' }],
    { duration: 220, easing: 'ease-in' }).onfinish = () => {
      swap();
      el.animate([{ transform: 'translateY(8px)', opacity: 0, filter: 'blur(3px)' }, { transform: 'none', opacity: 1, filter: 'none' }],
        { duration: 320, easing: 'cubic-bezier(.2,.8,.2,1)' });
    };
}, 1000);

/* ---------- ввод ---------- */
isl.addEventListener('mouseenter', () => { hover = true; clearTimeout(hoverT); hoverT = setTimeout(() => { if (hover && !S.hidden) { clearTimeout(popT); setView('vFull'); } }, 110); });
isl.addEventListener('mouseleave', () => { hover = false; clearTimeout(hoverT); hoverT = setTimeout(() => { if (!hover && !S.pinned) S.waiting ? showWaiting() : setView('vCompact'); }, 220); });
// WebView2 иногда теряет mouseleave при быстром уходе курсора — проверяем позицию сами
setInterval(async () => {
  if (!TAURI || !hover || S.pinned || !isl.classList.contains('open')) return;
  if (!(await invoke('cursor_inside'))) { hover = false; S.waiting ? showWaiting() : setView('vCompact'); }
}, 500);
isl.addEventListener('click', e => {
  if (S.hidden) { unhide(); return; }
  if (isl.classList.contains('notif')) { clearTimeout(popT); setView('vFull'); return; }
  if (e.target.closest('#tTg') && S.unread.length) { S.unread = []; invoke('mark_tg_read'); render(); }
});
isl.addEventListener('contextmenu', e => { e.preventDefault(); S.hidden ? unhide() : hide(); });
isl.addEventListener('keydown', e => {
  if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle(); }
  if (e.key === 'Escape') hide();
});
$('bHide').onclick = e => { e.stopPropagation(); hide(); };
$('bSettings').onclick = e => { e.stopPropagation(); invoke('open_settings'); };
$('mPlay').onclick = e => {
  e.stopPropagation(); invoke('media_control', { action: 'toggle' });
  pop($('mPlay'), .5);
  if (!RM) $('cover').animate([{ transform: `scale(${S.media.playing ? .88 : 1})` }, { transform: `scale(${S.media.playing ? 1 : .88})` }],
    { duration: 450, easing: 'cubic-bezier(.34,1.5,.5,1)', fill: 'forwards' });
};
$('mNext').onclick = e => { e.stopPropagation(); invoke('media_control', { action: 'next' }); };
$('mPrev').onclick = e => { e.stopPropagation(); invoke('media_control', { action: 'prev' }); };

/* ---------- приём данных ---------- */
function applyUsage(u) {
  const prev = S.resetAt;
  const ms = v => v ? (+new Date(v) || 0) : 0;   // ISO из Rust или число из демо
  S.ok = !!u.ok; S.err = u.error || '';
  if (!S.ok) { render(); return; }                // ошибку показываем, старые цифры не трогаем
  S.week = Math.round(u.week_pct ?? 0); S.weekResetAt = ms(u.week_reset_at);
  S.resetAt = ms(u.reset_at); S.lastUpdate = Date.now();
  const p = Math.round(u.pct ?? 0);

  if (prev && S.resetAt > prev) {                   // окно сбросилось
    S.warned = {}; S.hist = [[Date.now(), p]];
    fx('celebrate', 1400);   // без автораскрытия — только вспышка
  }
  if (p !== S.pct) { S.hist.push([Date.now(), p]); if (S.hist.length > 400) S.hist.splice(0, S.hist.length - 400); }
  S.pct = p; tweenTo(p);
  try { localStorage.setItem('ci-hist', JSON.stringify(S.hist)); } catch {}

  for (const t of [CFG.warn, CFG.alarm]) {
    if (p >= t && !S.warned[t]) { S.warned[t] = 1; fx(t >= CFG.alarm ? 'alarm' : 'warn', 1600); }
    if (p < t) S.warned[t] = 0;
  }
  render();
}

// Акцент берём из обложки: средний цвет по насыщенным пикселям, серое отбрасываем.
function accentFrom(src) {
  return new Promise(res => {
    const img = new Image();
    img.onload = () => {
      const c = document.createElement('canvas'); c.width = c.height = 16;
      const g = c.getContext('2d', { willReadFrequently: true });
      g.drawImage(img, 0, 0, 16, 16);
      let r = 0, gg = 0, b = 0, n = 0;
      try {
        const d = g.getImageData(0, 0, 16, 16).data;
        for (let i = 0; i < d.length; i += 4) {
          const mx = Math.max(d[i], d[i + 1], d[i + 2]), mn = Math.min(d[i], d[i + 1], d[i + 2]);
          if (mx - mn < 24) continue;
          r += d[i]; gg += d[i + 1]; b += d[i + 2]; n++;
        }
      } catch { return res(''); }
      res(n ? `rgb(${Math.round(r / n)},${Math.round(gg / n)},${Math.round(b / n)})` : '');
    };
    img.onerror = () => res('');
    img.src = src;
  });
}

// свечение при play/pause: волна идёт от эквалайзера (в свёрнутом виде) или от кнопки ▶ (в карточке)
function mediaGlow(playing) {
  if (RM || S.hidden || !on('fx_media')) return;
  const g = $('mGlow'), src = isl.classList.contains('open') ? $('mPlay') : isl.querySelector('.compact .eq');
  const r = src && src.getBoundingClientRect(), ir = isl.getBoundingClientRect();
  g.style.setProperty('--mx', r && r.width ? ((r.left + r.width / 2 - ir.left) / ir.width * 100).toFixed(1) + '%' : '50%');
  g.className = 'mglow'; void g.offsetWidth;
  g.className = 'mglow ' + (playing ? 'play' : 'pause');
}

// обложка приходит отдельным событием и только при смене трека; картинку и цвет
// применяем один раз, а не при каждом обновлении позиции (раньше — каждые 1–5 с)
let coverId = -1;
function applyCover(c) {
  if (!c || c.id === coverId) return;
  coverId = c.id;
  S.media.cover = c.cover || '';
  const cv = $('cover');
  cv.style.backgroundImage = S.media.cover ? `url(${S.media.cover})` : '';
  cv.textContent = S.media.cover ? '' : (S.media.title ? S.media.title[0] : '♪');
  if (S.media.cover) accentFrom(S.media.cover).then(a => { if (a) { S.media.accent = a; isl.style.setProperty('--eqc', a); } });
  else { S.media.accent = ''; isl.style.removeProperty('--eqc'); }
}
listen('media-cover', e => applyCover(e.payload));

function applyMedia(m) {
  const changed = m.title && m.title !== S.media.title;
  // Яндекс Музыка и др. редко обновляют позицию: точку отсчёта двигаем только когда позиция
  // реально пришла новая (или сменился трек / пауза), иначе повторы событий «замораживали» время
  if (S.media.has && m.has && m.title === S.media.title && !!m.playing !== S.media.playing) mediaGlow(!!m.playing);
  const o = S.media, same = m.title === o.title && !!m.playing === o.playing && (m.pos || 0) === o.pos;
  S.media = { has: !!m.has, playing: !!m.playing, title: m.title || '', artist: m.artist || '', pos: m.pos || 0, dur: m.dur || 0, cover: o.cover || '', accent: o.accent || '', posAt: same && o.posAt ? o.posAt : Date.now() };
  const cv = $('cover');
  if (!S.media.cover) cv.textContent = S.media.title ? S.media.title[0] : '♪';
  // обложка пропущена (окно загрузилось позже события) — забираем её сами
  if (TAURI && m.cover_id !== undefined && m.cover_id !== coverId) invoke('get_media_cover').then(applyCover).catch(() => {});
  if (changed && !RM) {
    cv.animate([{ transform: 'translateX(14px) rotateY(60deg)', opacity: 0 }, { transform: 'none', opacity: 1 }], { duration: 450, easing: 'cubic-bezier(.2,.8,.2,1)' });
    $('song').animate([{ transform: 'translateX(10px)', opacity: 0 }, { transform: 'none', opacity: 1 }], { duration: 350, easing: 'ease-out' });
  }
  render();
}

// в приложении список непрочитанных приходит целиком (tg-state), здесь только анимация
function applyNotif(msg) {
  if (!TAURI) S.unread.push(msg);
  render();
  if (S.hidden) return;
  const n = S.unread.length;
  $('nFrom').textContent = n > 1 ? t('{0} и ещё {1}', msg.from, n - 1) : msg.from;
  $('nText').textContent = msg.text;
  setView('vNotif'); fx('bump', 500); pop($('dot'), 0);
  clearTimeout(popT);
  popT = setTimeout(() => setView(hover || S.pinned ? 'vFull' : 'vCompact'), 3000);
}

listen('usage-update', e => applyUsage(e.payload));
listen('media-update', e => applyMedia(e.payload));
listen('tg-state', e => { S.tg = e.payload.status; S.unread = e.payload.items || []; render(); });
listen('tg-notification', e => applyNotif(e.payload));
// светящаяся линия по нижнему краю: громкость (белая), зарядка (зелёная), батарея (красная)
let lineT;
function line(pct, kind = '', ms = 1400) {
  if (S.hidden) return;
  const g = $('volGlow');
  g.className = 'volglow on ' + kind;
  $('vFill').style.width = pct + '%';
  clearTimeout(lineT);
  lineT = setTimeout(() => g.classList.remove('on'), ms);
}
// громкость: линия по краю + процент на месте часов, пока линия горит
function applyVolume(v) {
  if (!on('fx_volume')) return;
  v.muted ? line(100, 'muted') : line(v.level);
  S.volText = v.muted || !v.level ? '🔇 0%' : (v.level < 34 ? '🔈 ' : v.level < 67 ? '🔉 ' : '🔊 ') + v.level + '%';
  S.volUntil = Date.now() + 1400;
  $('cTimer').textContent = S.volText;
  isl.classList.add('vol');
  clearTimeout(applyVolume.t);
  applyVolume.t = setTimeout(() => { isl.classList.remove('vol'); $('cTimer').textContent = compactText(S.resetAt - Date.now()); }, 1400);
}

// плашка: короткое сообщение вместо свёрнутого вида; ms = 0 — висит, пока не уберут
let chipT;
function chip(text, ico, kind, ms = 2200) {
  if (S.hidden || isl.classList.contains('open') || isl.classList.contains('notif')) return;
  $('cText').textContent = text;
  $('cIco').textContent = ico;
  $('vChip').className = 'view chipview ' + kind + ($('vChip').classList.contains('on') ? ' on' : '');
  if (!isl.classList.contains('chip')) { setView('vChip'); fx('bump', 400); } else { fit(); pop($('cIco'), .7); }
  clearTimeout(chipT);
  if (ms) chipT = setTimeout(chipEnd, ms);
}
function chipEnd() {
  if (S.waiting) return showWaiting();
  if (isl.classList.contains('chip')) setView(hover ? 'vFull' : 'vCompact');
}
function showWaiting() { chip(t('Claude ждёт разрешения'), '!', 'amber', 0); }

listen('waiting', e => {
  const w = !!e.payload && on('fx_wait');
  if (w === !!S.waiting) return;
  S.waiting = w; render();
  if (w) showWaiting(); else if (isl.classList.contains('chip') && $('vChip').classList.contains('amber')) chipEnd();
});
listen('claude-done', () => { if (S.waiting || !on('fx_done')) return; fx('done', 1600); chip(t('Claude закончил'), '✓', 'green', 2600); });
listen('copied', () => on('fx_copy') && chip(t('Скопировано'), '⧉', 'white', 1300));
listen('audio-device', e => on('fx_audio') && chip(e.payload, '🎧', 'white', 2600));
listen('power', e => {
  const p = e.payload;
  if (!on('fx_power')) return;
  if (p.low) { line(p.pct, 'low', 2600); chip(t('Батарея {0}% — подключи зарядку', p.pct), '!', 'red', 3500); }
  else if (p.charging) { line(p.pct, 'charge', 2600); chip(t('Зарядка · {0}%', p.pct), '⚡', 'green', 2600); }
  else chip(t('От батареи · {0}%', p.pct), '🔋', 'white', 2000);
});
// раскладка: на мгновение показываем язык
listen('layout', e => on('fx_layout') && chip(e.payload, '⌨', 'white', 900));

// камера и микрофон: точка на острове, пока используются; плашка — в момент включения
let priv = { cam: false, mic: false };
listen('privacy', e => {
  const p = e.payload, d = $('pdot'), show = on('fx_privacy');
  d.className = 'pdot' + (show && p.cam ? ' cam' : show && p.mic ? ' mic' : '');
  if (show && p.cam && !priv.cam) chip(t('Камера · {0}', p.app), '●', 'green', 2200);
  else if (show && p.mic && !priv.mic && !p.cam) chip(t('Микрофон · {0}', p.app), '●', 'amber', 2200);
  priv = p; fit();
});

// помодоро: кнопка ⏱ в карточке; обратный отсчёт заменяет часы в свёрнутом виде
let pomoT;
function pomoToggle() {
  clearTimeout(pomoT);
  if (S.pomoEnd) { S.pomoEnd = 0; chip(t('Таймер остановлен'), '⏱', 'white', 1400); }
  else {
    S.pomoEnd = Date.now() + CFG.pomo * 60e3;
    pomoT = setTimeout(pomoDone, CFG.pomo * 60e3);
    setView('vCompact'); chip(t('Помодоро · {0} мин', CFG.pomo), '🍅', 'red', 1600);
  }
  isl.classList.toggle('pomo', !!S.pomoEnd); render();
}
function pomoDone() {
  S.pomoEnd = 0; isl.classList.remove('pomo'); render();
  fx('done', 1600); chip(t('Время вышло — сделай перерыв'), '🍅', 'green', 6000); beep();
}
function beep() {
  try {
    const a = new AudioContext(), o = a.createOscillator(), g = a.createGain();
    o.frequency.value = 880; g.gain.setValueAtTime(.0001, a.currentTime);
    g.gain.exponentialRampToValueAtTime(.15, a.currentTime + .02); g.gain.exponentialRampToValueAtTime(.0001, a.currentTime + .6);
    o.connect(g).connect(a.destination); o.start(); o.stop(a.currentTime + .65);
  } catch {}
}
$('bTimer').onclick = e => { e.stopPropagation(); pomoToggle(); };

listen('update-available', e => chip(t('Доступно обновление {0}', e.payload), '↑', 'white', 4000));

listen('toggle-hide', () => { S.hidden ? unhide() : hide(); });

// перемотка: клик по полоске трека
$('prog').parentElement.addEventListener('click', e => {
  e.stopPropagation();
  const m = S.media;
  if (!m.has || !m.dur) return;
  const r = e.currentTarget.getBoundingClientRect();
  const sec = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * m.dur;
  invoke('media_control', { action: 'seek:' + sec.toFixed(1) });
  S.media = { ...m, pos: sec, posAt: Date.now() };
  render();
});
listen('volume', e => applyVolume(e.payload));
listen('thinking', e => { S.thinking = !!e.payload; render(); });
// настройки: и при запуске (события из setup приходят раньше, чем мы начинаем слушать), и при изменении
function applySettings(c) {
  if (!c) return;
  if (c.pos) stage.dataset.pos = c.pos;
  document.documentElement.dataset.theme = c.theme || 'black';
  CFG.warn = c.warn_at || 80; CFG.alarm = c.alarm_at || 95; CFG.swap = Math.max(2, c.swap_secs || 4);
  if (c.opacity != null) document.documentElement.style.setProperty('--island-a', c.opacity);
  $('tMusic').hidden = c.show_music === false; fit(); CFG.tg = c.show_telegram !== false;
  CFG.fx = c; CFG.pomo = c.pomodoro_min || 25;
  setLang(c.lang); lockWidth();
  render();
}
listen('settings-update', e => applySettings(e.payload));
if (TAURI) invoke('get_settings').then(applySettings).catch(() => {});
listen('toggle-hidden', () => { S.hidden ? unhide() : hide(); });
listen('open-settings', () => invoke('open_settings'));

render(); lockWidth();

/* ---------- демо-режим без Tauri (только для просмотра визуала в браузере) ---------- */
if (!TAURI) {
  document.body.style.background = 'linear-gradient(135deg,#1d2a44,#3b2f5c)';
  let p = 58;
  applyUsage({ ok: true, pct: p, week_pct: 38, reset_at: Date.now() + 83 * 60e3, week_reset_at: Date.now() + 3 * 864e5 });
  applyMedia({ has: true, playing: true, title: 'Midnight City', artist: 'M83', pos: 57, dur: 244 });
  isl.style.setProperty('--eqc', '#fa2d48');
  setInterval(() => { S.media.pos = (S.media.pos + 1) % S.media.dur; S.media.posAt = Date.now(); }, 1000);
  setTimeout(() => applyNotif({ from: 'Аня', text: 'скинь файл когда сможешь' }), 4000);
  setInterval(() => { S.thinking = !S.thinking; render(); }, 6000);   // демо «думает»
  setInterval(() => { p = Math.min(100, p + 3); applyUsage({ ok: true, pct: p, week_pct: 38, reset_at: Date.now() + 70 * 60e3, week_reset_at: Date.now() + 3 * 864e5 }); }, 15000);
}
