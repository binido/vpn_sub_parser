// Тонкая обёртка над window.__TAURI__ (withGlobalTauri), чтобы UI можно было
// открыть и в обычном браузере — там часть возможностей просто недоступна.
const api = window.__TAURI__ ?? {};

const notInTauri = () =>
  Promise.reject(new Error('Действие доступно только в приложении'));

export const invoke = api.core?.invoke ?? notInTauri;

export async function openUrl(url) {
  if (api.opener) return api.opener.openUrl(url);
  window.open(url, '_blank');
}

export async function copyText(text) {
  if (api.clipboardManager) return api.clipboardManager.writeText(text);
  return navigator.clipboard.writeText(text);
}

export async function saveTextFile(contents, defaultName) {
  if (!api.dialog || !api.fs) return notInTauri();
  const path = await api.dialog.save({
    defaultPath: defaultName,
    filters: [{ name: 'JSON', extensions: ['json'] }],
  });
  if (!path) return null;
  await api.fs.writeTextFile(path, contents);
  return path;
}
