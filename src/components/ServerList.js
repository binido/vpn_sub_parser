import { ServerCard } from './ServerCard.js';
import { ExportButton } from './ExportButton.js';

export function ServerList(servers, { onToast }) {
  const el = document.createElement('section');
  el.className = 'server-list';
  el.innerHTML = `
    <div class="server-list__bar">
      <h2 class="server-list__counter"></h2>
      <div class="server-list__actions"></div>
    </div>
    <div class="server-list__items"></div>`;

  el.querySelector('.server-list__counter').textContent =
    `Найдено серверов: ${servers.length}`;
  el.querySelector('.server-list__actions').append(ExportButton(servers, { onToast }));

  const items = el.querySelector('.server-list__items');
  servers.forEach((server, i) => items.append(ServerCard(server, i, { onToast })));

  return el;
}
