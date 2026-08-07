import { attachRipple } from './ripple.js';
import { openUrl } from '../tauri.js';

export function UpdateBanner({ version, url }) {
  const el = document.createElement('section');
  el.className = 'update-banner';
  el.innerHTML = `
    <span class="update-banner__text"></span>
    <button type="button" class="m3-button m3-button--tonal">
      <span class="m3-button__label">Открыть релиз</span>
    </button>`;

  el.querySelector('.update-banner__text').textContent = `Доступна версия ${version}`;
  const button = el.querySelector('button');
  attachRipple(button);
  button.addEventListener('click', () => openUrl(url));

  return el;
}
