import { useEffect, useState } from 'react';
import { Sun, Moon, Monitor } from 'lucide-react';
export type Theme = 'system' | 'light' | 'dark';
function saved(): Theme {
  const value = localStorage.getItem('pg-theme');
  return value === 'light' || value === 'dark' ? value : 'system';
}
function apply() {
  const theme = saved();
  document.documentElement.dataset.theme =
    theme === 'system'
      ? matchMedia('(prefers-color-scheme: dark)').matches
        ? 'dark'
        : 'light'
      : theme;
}
apply();
export function ThemeControl() {
  const [theme, setTheme] = useState<Theme>(saved);
  useEffect(() => {
    const query = matchMedia('(prefers-color-scheme: dark)');
    query.addEventListener('change', apply);
    const update = () => {
      setTheme(saved());
      apply();
    };
    window.addEventListener('pg-theme-change', update);
    return () => {
      query.removeEventListener('change', apply);
      window.removeEventListener('pg-theme-change', update);
    };
  }, []);
  return (
    <div className="theme-control" role="group" aria-label="Appearance">
      {(
        [
          { id: 'system', icon: Monitor, label: 'System' },
          { id: 'light', icon: Sun, label: 'Light' },
          { id: 'dark', icon: Moon, label: 'Dark' },
        ] as const
      ).map((t) => (
        <button
          key={t.id}
          type="button"
          aria-pressed={theme === t.id}
          onClick={() => {
            localStorage.setItem('pg-theme', t.id);
            window.dispatchEvent(new Event('pg-theme-change'));
          }}
        >
          <t.icon size={16} />
          {t.label}
        </button>
      ))}
    </div>
  );
}

const accents = [
  { name: 'Garage lime', value: '#c2f218' },
  { name: 'Copper', value: '#f5aa63' },
  { name: 'Sky', value: '#76c8ff' },
  { name: 'Violet', value: '#c3a0ff' },
  { name: 'Rose', value: '#ff94b1' },
  { name: 'Teal', value: '#68dfc4' },
];
function accentValue() {
  const v = localStorage.getItem('pg-accent') || '#c2f218';
  return /^#[0-9a-f]{6}$/i.test(v) ? v : '#c2f218';
}
function applyAccent(value: string) {
  const rgb = [1, 3, 5].map((i) => parseInt(value.slice(i, i + 2), 16) / 255);
  const luminance = rgb
    .map((v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4))
    .reduce((s, v, i) => s + v * [0.2126, 0.7152, 0.0722][i], 0);
  const root = document.documentElement;
  root.style.setProperty('--lime', value);
  root.style.setProperty('--accent', value);
  root.style.setProperty('--accent-ink', luminance > 0.179 ? '#111711' : '#ffffff');
}
applyAccent(accentValue());
export function AccentControl() {
  const [value, setValue] = useState(accentValue);
  function change(v: string) {
    setValue(v);
    localStorage.setItem('pg-accent', v);
    applyAccent(v);
  }
  return (
    <div className="accent-control">
      <h3>Accent color</h3>
      <p>Buttons, progress, and highlights. Saved on this device.</p>
      <div className="accent-swatches" role="group" aria-label="Accent color">
        {accents.map((a) => (
          <button
            type="button"
            key={a.value}
            title={a.name}
            aria-label={a.name}
            aria-pressed={value === a.value}
            style={{ background: a.value, color: '#111' }}
            onClick={() => change(a.value)}
          >
            {value === a.value ? '✓' : ''}
          </button>
        ))}
      </div>
      <label className="custom-accent">
        Custom color
        <input type="color" value={value} onChange={(e) => change(e.target.value)} />
        <span>{value.toUpperCase()}</span>
      </label>
    </div>
  );
}
