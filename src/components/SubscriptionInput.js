import { attachRipple } from './ripple.js';

export function SubscriptionInput({ onParse }) {
  const el = document.createElement('section');
  el.className = 'surface-card input-card';
  el.innerHTML = `
    <h2 class="input-card__title">Ссылка подписки</h2>
    <div class="input-card__row">
      <label class="text-field">
        <input type="url" spellcheck="false" autocomplete="off"
               placeholder="https://panel.example.com/sub/abc123" />
        <span class="text-field__label">URL подписки</span>
      </label>
      <button type="button" class="m3-button m3-button--filled parse-btn">
        <span class="spinner" hidden></span>
        <span class="m3-button__label">Распарсить</span>
      </button>
    </div>
    <p class="error-banner" hidden></p>`;

  const input = el.querySelector('input');
  const button = el.querySelector('.parse-btn');
  const spinner = el.querySelector('.spinner');
  const error = el.querySelector('.error-banner');
  attachRipple(button);

  const submit = () => onParse(input.value.trim());
  button.addEventListener('click', submit);
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') submit();
  });

  return {
    el,
    get value() {
      return input.value.trim();
    },
    set value(v) {
      input.value = v ?? '';
    },
    setLoading(loading) {
      button.disabled = loading;
      spinner.hidden = !loading;
      button.querySelector('.m3-button__label').textContent = loading
        ? 'Загрузка…'
        : 'Распарсить';
    },
    setError(message) {
      error.textContent = message ?? '';
      error.hidden = !message;
    },
  };
}
