export function burstParticles(el: HTMLElement) {
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  const rect = el.getBoundingClientRect();
  const cx = rect.left + rect.width / 2;
  const cy = rect.top + rect.height / 2;
  const count = 7;
  for (let i = 0; i < count; i++) {
    const p = document.createElement('span');
    p.className = 'toggle-burst-particle';
    const angle = (Math.PI * 2 / count) * i + (Math.random() - 0.5) * 0.8;
    const dist = 12 + Math.random() * 14;
    const size = 2.5 + Math.random() * 2;
    p.style.setProperty('--tx', `${Math.cos(angle) * dist}px`);
    p.style.setProperty('--ty', `${Math.sin(angle) * dist}px`);
    p.style.width = `${size}px`;
    p.style.height = `${size}px`;
    p.style.left = `${cx}px`;
    p.style.top = `${cy}px`;
    p.style.animationDelay = `${Math.random() * 40}ms`;
    document.body.appendChild(p);
    p.addEventListener('animationend', () => p.remove());
  }
}
