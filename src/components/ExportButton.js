import { attachRipple } from './ripple.js';
import { invoke, saveTextFile } from '../tauri.js';

export function ExportButton(servers, { onToast }) {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'm3-button m3-button--outlined';
  button.innerHTML = '<span class="m3-button__label">Экспорт в Xray outbounds</span>';
  attachRipple(button);

  button.addEventListener('click', async () => {
    button.disabled = true;
    try {
      const json = await invoke('export_xray_outbounds', { servers });
      const path = await saveTextFile(json, 'xray-outbounds.json');
      if (path) onToast('Сохранено: ' + path);
    } catch (e) {
      onToast(`Не удалось сохранить файл: ${e.message ?? e}`);
    } finally {
      button.disabled = false;
    }
  });

  return button;
}
