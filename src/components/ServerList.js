import { ServerCard } from './ServerCard.js';
import { ExportButton } from './ExportButton.js';
import { attachRipple } from './ripple.js';
import { invoke } from '../tauri.js';

export function ServerList(servers, { onToast }) {
  const el = document.createElement('section');
  el.className = 'server-list';
  el.innerHTML = `
    <div class="server-list__bar">
      <h2 class="server-list__counter"></h2>
      <div class="server-list__actions">
        <button type="button" class="m3-button m3-button--tonal ping-btn">
          <span class="spinner" hidden></span>
          <span class="m3-button__label">Проверить пинг</span>
        </button>
      </div>
    </div>
    <div class="list-controls">
      <label class="text-field text-field--compact">
        <input type="search" spellcheck="false" placeholder="например: Нидерланды" />
        <span class="text-field__label">Фильтр по имени</span>
      </label>
      <label class="select-field">
        <span>Сортировка</span>
        <select>
          <option value="original">как в подписке</option>
          <option value="ping">по пингу</option>
          <option value="name">по имени</option>
        </select>
      </label>
    </div>
    <div class="server-list__items"></div>`;

  const counter = el.querySelector('.server-list__counter');
  const items = el.querySelector('.server-list__items');
  const search = el.querySelector('input');
  const sort = el.querySelector('select');

  el.querySelector('.server-list__actions').append(ExportButton(servers, { onToast }));

  function render() {
    const query = search.value.trim().toLowerCase();
    let visible = servers.filter((s) => s.name.toLowerCase().includes(query));

    if (sort.value === 'name') {
      visible = [...visible].sort((a, b) => a.name.localeCompare(b.name, 'ru'));
    } else if (sort.value === 'ping') {
      // не ответившие и непроверенные уезжают в конец
      visible = [...visible].sort((a, b) => (a.ping ?? Infinity) - (b.ping ?? Infinity));
    }

    counter.textContent =
      visible.length === servers.length
        ? `Найдено серверов: ${servers.length}`
        : `Показано ${visible.length} из ${servers.length}`;
    items.replaceChildren(
      ...visible.map((server, i) => ServerCard(server, i, { onToast }))
    );
  }

  search.addEventListener('input', render);
  sort.addEventListener('change', render);

  const pingButton = el.querySelector('.ping-btn');
  const spinner = pingButton.querySelector('.spinner');
  const pingLabel = pingButton.querySelector('.m3-button__label');
  attachRipple(pingButton);
  pingButton.addEventListener('click', async () => {
    pingButton.disabled = true;
    spinner.hidden = false;
    pingLabel.textContent = 'Проверяю…';
    try {
      const latencies = await invoke('ping_servers', { servers });
      latencies.forEach((ms, i) => {
        servers[i].ping = ms ?? null;
      });
      const alive = latencies.filter((ms) => ms !== null).length;
      onToast(`Ответили ${alive} из ${servers.length}`);
      sort.value = 'ping';
      render();
    } catch (e) {
      onToast(`Не удалось проверить: ${e.message ?? e}`);
    } finally {
      pingButton.disabled = false;
      spinner.hidden = true;
      pingLabel.textContent = 'Проверить пинг';
    }
  });

  render();
  return el;
}
