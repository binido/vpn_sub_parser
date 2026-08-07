// Material-ripple: одна волна от точки нажатия.
export function attachRipple(el) {
  el.classList.add('ripple-host');
  el.addEventListener('pointerdown', (event) => {
    const rect = el.getBoundingClientRect();
    const size = Math.max(rect.width, rect.height) * 2;
    const wave = document.createElement('span');
    wave.className = 'ripple';
    wave.style.width = wave.style.height = `${size}px`;
    wave.style.left = `${event.clientX - rect.left - size / 2}px`;
    wave.style.top = `${event.clientY - rect.top - size / 2}px`;
    wave.addEventListener('animationend', () => wave.remove());
    el.appendChild(wave);
  });
}
