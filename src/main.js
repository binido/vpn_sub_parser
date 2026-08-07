import { SubscriptionInput } from './components/SubscriptionInput.js';
import { ServerList } from './components/ServerList.js';
import { attachRipple } from './components/ripple.js';
import { invoke, copyText } from './tauri.js';

const LAST_URL_KEY = 'vpn-sub-parser:last-url';
const THEME_KEY = 'vpn-sub-parser:theme';

const toastEl = document.getElementById('toast');
let toastTimer;
function toast(message) {
  toastEl.textContent = message;
  toastEl.classList.add('toast--visible');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toastEl.classList.remove('toast--visible'), 3500);
}

// Тема: сохранённый выбор, иначе системная.
const themeToggle = document.getElementById('theme-toggle');
const themeIcon = document.getElementById('theme-icon');
const systemDark = window.matchMedia('(prefers-color-scheme: dark)');
function applyTheme(theme) {
  document.documentElement.dataset.theme = theme;
  themeIcon.textContent = theme === 'dark' ? '☀' : '☾';
}
applyTheme(localStorage.getItem(THEME_KEY) ?? (systemDark.matches ? 'dark' : 'light'));
systemDark.addEventListener('change', (e) => {
  if (!localStorage.getItem(THEME_KEY)) applyTheme(e.matches ? 'dark' : 'light');
});
attachRipple(themeToggle);
themeToggle.addEventListener('click', () => {
  const next = document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark';
  localStorage.setItem(THEME_KEY, next);
  applyTheme(next);
});

const resultsSlot = document.getElementById('results-slot');
const fabSlot = document.getElementById('fab-slot');

function renderResults(servers) {
  resultsSlot.replaceChildren(ServerList(servers, { onToast: toast }));

  const fab = document.createElement('button');
  fab.type = 'button';
  fab.className = 'fab';
  fab.innerHTML = '<span>⧉</span><span>Копировать все</span>';
  attachRipple(fab);
  fab.addEventListener('click', async () => {
    try {
      await copyText(servers.map((s) => s.raw).join('\n'));
      toast(`Скопировано ссылок: ${servers.length}`);
    } catch (e) {
      toast(`Не удалось скопировать: ${e.message ?? e}`);
    }
  });
  fabSlot.replaceChildren(fab);
}

const input = SubscriptionInput({
  async onParse(url) {
    input.setError(null);
    if (!url) return input.setError('Введите ссылку подписки');
    input.setLoading(true);
    try {
      const servers = await invoke('parse_subscription', { url });
      localStorage.setItem(LAST_URL_KEY, url);
      renderResults(servers);
    } catch (e) {
      resultsSlot.replaceChildren();
      fabSlot.replaceChildren();
      input.setError(typeof e === 'string' ? e : (e.message ?? 'Неизвестная ошибка'));
    } finally {
      input.setLoading(false);
    }
  },
});

// Последний URL подставляется, но запрос не выполняется автоматически.
input.value = localStorage.getItem(LAST_URL_KEY) ?? '';
document.getElementById('input-slot').append(input.el);
