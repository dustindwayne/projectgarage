import { useEffect, useState } from 'react';
import { Smartphone, RefreshCw, Wifi } from 'lucide-react';
import { native } from './api';
type Nearby = {
  platform: string;
  enabled: boolean;
  peers: { id: string; address: string; name: string }[];
  error: string | null;
};
export function NearbyDevices() {
  const [state, setState] = useState<Nearby | null>(null),
    [error, setError] = useState('');
  async function scan() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      setState(await invoke<Nearby>('nearby_devices'));
      setError('');
    } catch (e) {
      setError(String(e));
    }
  }
  useEffect(() => {
    if (!native) return;
    void scan();
    const timer = setInterval(() => void scan(), 4000);
    return () => clearInterval(timer);
  }, []);
  if (!native) return null;
  return (
    <section className="settings-card">
      <h2>
        <Wifi size={19} />
        {state?.platform === 'android' ? 'Nearby on Wi-Fi' : 'Nearby Android apps'}
      </h2>
      <p>
        {state?.platform === 'android'
          ? 'While this app is open, desktop installations on the same Wi-Fi can discover it. Only an app identifier is announced; your garage records stay on your phone.'
          : 'This desktop app automatically looks for Project Garage on Android devices using the same Wi-Fi. Keep the Android app open. Guest networks and firewalls may block discovery.'}
      </p>
      <label className="checkbox">
        <input
          type="checkbox"
          checked={state?.enabled ?? true}
          onChange={async (e) => {
            try {
              const { invoke } = await import('@tauri-apps/api/core');
              setState(await invoke<Nearby>('set_nearby_enabled', { enabled: e.target.checked }));
            } catch (e) {
              setError(String(e));
            }
          }}
        />
        Allow Wi-Fi discovery
      </label>
      {(error || state?.error) && (
        <p role="alert" className="form-error">
          {error || state?.error}
        </p>
      )}
      {state?.platform !== 'android' && (
        <>
          <p className="muted" role="status">
            {!state?.enabled
              ? 'Discovery off'
              : state.peers.length
                ? `${state.peers.length} Android app${state.peers.length === 1 ? '' : 's'} found`
                : 'Looking for Android apps…'}
          </p>
          {state?.peers.map((peer) => (
            <div className="member" key={peer.id}>
              <Smartphone size={22} />
              <div>
                <strong>{peer.name}</strong>
                <small>{peer.address} · Same Wi-Fi</small>
              </div>
              <span className="pill installed">Nearby</span>
            </div>
          ))}
          <button className="secondary" onClick={() => void scan()}>
            <RefreshCw size={16} />
            Check again
          </button>
        </>
      )}
      <small className="settings-help">
        Discovery identifies nearby apps. It does not pair devices, transfer records, or merge
        garages.
      </small>
    </section>
  );
}
