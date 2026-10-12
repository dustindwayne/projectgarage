import { createContext, useContext, useEffect, useState } from 'react';
import { Download, RefreshCw, ShieldCheck } from 'lucide-react';
import { native } from './api';
type Update = {
  current_version: string;
  latest_version: string;
  available: boolean;
  notes: string;
  platform: string;
  package_available: boolean;
};
type State = {
  installedVersion: string;
  info: Update | null;
  busy: boolean;
  error: string;
  message: string;
  check: () => Promise<void>;
  install: () => Promise<void>;
};
const Context = createContext<State | null>(null);
export function UpdatesProvider({ children }: { children: React.ReactNode }) {
  const [info, setInfo] = useState<Update | null>(null),
    [installedVersion, setInstalledVersion] = useState('Loading…'),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(''),
    [message, setMessage] = useState('');
  async function check() {
    localStorage.setItem('pg-last-update-check', String(Date.now()));
    setBusy(true);
    setError('');
    setMessage('');
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<Update>('check_github_update');
      setInfo(result);
      setMessage(
        result.available
          ? result.package_available
            ? 'A new version is ready.'
            : 'A new version exists, but no installer is published for this device.'
          : 'You have the latest version.',
      );
      localStorage.setItem('pg-last-update-check', String(Date.now()));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function install() {
    setBusy(true);
    setError('');
    setMessage('Downloading and verifying your update…');
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<string>('install_github_update');
      setMessage(
        result === 'permission_required'
          ? 'Allow updates from Project Garage in Android settings, return here, then tap Download & install again.'
          : result === 'restarting'
            ? 'Installing and restarting Project Garage…'
            : 'The Android installer is open. Confirm the update there. Your garage records stay on this device.',
      );
    } catch (e) {
      setMessage('');
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    if (native)
      void import('@tauri-apps/api/app')
        .then((api) => api.getVersion())
        .then(setInstalledVersion)
        .catch(() => setInstalledVersion('Unavailable'));
    if (
      native &&
      localStorage.getItem('pg-auto-updates') !== 'false' &&
      Date.now() - Number(localStorage.getItem('pg-last-update-check') || 0) > 4 * 60 * 60 * 1000
    )
      void check();
  }, []);
  return (
    <Context.Provider value={{ installedVersion, info, busy, error, message, check, install }}>
      {children}
    </Context.Provider>
  );
}
export function UpdatesBanner({ onOpen }: { onOpen: () => void }) {
  const state = useContext(Context);
  if (!state?.info?.available) return null;
  return (
    <div className="offline-banner">
      <Download size={17} />
      <span>Project Garage {state.info.latest_version} is available.</span>
      <button className="text-link" onClick={onOpen}>
        View update
      </button>
    </div>
  );
}
export function UpdatesCard() {
  const state = useContext(Context),
    [automatic, setAutomatic] = useState(localStorage.getItem('pg-auto-updates') !== 'false');
  if (!native || !state) return null;
  return (
    <section className="settings-card">
      <h2>
        <Download size={19} />
        App updates
      </h2>
      <p>
        Get new versions directly from Project Garage on GitHub. Installing an update keeps your
        garage records.
      </p>
      <p className="muted">
        Installed version: {state.info?.current_version || state.installedVersion}
      </p>
      <label className="checkbox">
        <input
          type="checkbox"
          checked={automatic}
          onChange={(e) => {
            setAutomatic(e.target.checked);
            localStorage.setItem('pg-auto-updates', String(e.target.checked));
          }}
        />
        Check for updates when opening the app
      </label>
      <p className="settings-help">Public update downloads. No GitHub account or token needed.</p>
      {state.error && (
        <p className="form-error" role="alert">
          {state.error}
        </p>
      )}
      {state.message && (
        <p className="muted" role="status">
          {state.message}
        </p>
      )}
      {state.info?.available && <p className="journal-notes">{state.info.notes}</p>}
      <div className="button-row">
        <button className="secondary" disabled={state.busy} onClick={() => void state.check()}>
          <RefreshCw size={16} />
          {state.busy ? 'Working…' : 'Check for updates'}
        </button>
        {state.info?.available && state.info.package_available && (
          <button className="primary" disabled={state.busy} onClick={() => void state.install()}>
            <Download size={16} />
            Download & install
          </button>
        )}
      </div>
      <small className="settings-help">
        <ShieldCheck size={14} /> Downloads are verified before installation. Android will ask you
        to approve the update.
      </small>
    </section>
  );
}
