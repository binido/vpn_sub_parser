import { attachRipple } from './ripple.js';
import { copyText } from '../tauri.js';

const ICONS = { vless: 'V', vmess: 'M', trojan: 'T', shadowsocks: 'S' };

// Значения полей приходят с чужого сервера — только textContent, никакого innerHTML.
export function ServerCard(server, index, { onToast }) {
  const el = document.createElement('article');
  el.className = 'server-card';
  el.style.animationDelay = `${Math.min(index, 15) * 35}ms`;
  el.innerHTML = `
    <button type="button" class="server-card__head" aria-expanded="false">
      <span class="proto-badge"></span>
      <span class="server-card__info">
        <span class="server-card__name"></span>
        <span class="server-card__address"></span>
        <span class="chips"></span>
      </span>
      <span class="chevron">⌄</span>
    </button>
    <div class="server-card__body">
      <div class="server-card__body-inner">
        <code class="raw-link"></code>
        <button type="button" class="m3-button m3-button--tonal copy-btn">
          <span class="m3-button__label">Копировать</span>
        </button>
      </div>
    </div>`;

  const badge = el.querySelector('.proto-badge');
  badge.textContent = ICONS[server.protocol] ?? '?';
  badge.classList.add(`proto-badge--${server.protocol}`);
  el.querySelector('.server-card__name').textContent = server.name;
  el.querySelector('.server-card__address').textContent = `${server.address}:${server.port}`;
  el.querySelector('.raw-link').textContent = server.raw;

  const d = server.details ?? {};
  const chips = [
    server.protocol,
    d.security && d.security !== 'none' ? d.security : null,
    d.network,
    d.flow?.includes('vision') ? 'vision' : d.flow,
    d.method,
  ].filter(Boolean);
  const chipBox = el.querySelector('.chips');
  for (const text of chips) {
    const chip = document.createElement('span');
    chip.className = 'chip';
    chip.textContent = text;
    chipBox.append(chip);
  }

  if (server.ping !== undefined) {
    const chip = document.createElement('span');
    const ms = server.ping;
    chip.className = `chip chip--${ms === null ? 'dead' : ms < 150 ? 'fast' : ms < 400 ? 'mid' : 'slow'}`;
    chip.textContent = ms === null ? 'нет ответа' : `${ms} мс`;
    chipBox.prepend(chip);
  }

  const head = el.querySelector('.server-card__head');
  attachRipple(head);
  head.addEventListener('click', () => {
    const expanded = el.classList.toggle('expanded');
    head.setAttribute('aria-expanded', String(expanded));
  });

  const copy = el.querySelector('.copy-btn');
  attachRipple(copy);
  copy.addEventListener('click', async () => {
    try {
      await copyText(server.raw);
      onToast('Ссылка скопирована');
    } catch (e) {
      onToast(`Не удалось скопировать: ${e.message ?? e}`);
    }
  });

  return el;
}
