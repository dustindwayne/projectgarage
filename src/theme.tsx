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
