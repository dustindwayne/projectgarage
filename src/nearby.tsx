import { useEffect, useState } from 'react';
import { Smartphone, RefreshCw, Wifi } from 'lucide-react';
import { native } from './api';
import type { Snapshot } from './types';
import { GarageImport } from './garage-import';
type Nearby = {
  platform: string;
  enabled: boolean;
  peers: { id: string; address: string; name: string }[];
  error: string | null;
};
type Transfer = {
  status: string;
  code: string;
  address: string;
  request_id: string;
  error: string;
};
export function NearbyDevices({
  snapshot,
  disabled,
  onImport,
}: {
  snapshot: Snapshot;
  disabled: boolean;
  onImport: (action: unknown) => Promise<void>;
}) {
  const [transfer, setTransfer] = useState<Transfer | null>(null),
    [incoming, setIncoming] = useState<Snapshot['garage'] | null>(null),
    [working, setWorking] = useState(false),
    [verified, setVerified] = useState(false);
  async function command(name: string, args?: Record<string, unknown>) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<any>(name, args);
  }
  const [state, setState] = useState<Nearby | null>(null),
    [error, setError] = useState('');
  async function scan() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      setState(await invoke<Nearby>('nearby_devices'));
      const t = await invoke<Transfer>('transfer_status');
      setTransfer(t);
      if (t.status === 'ready') {
        const text = await invoke<string>('take_phone_import');
        setIncoming(JSON.parse(text).garage);
      }
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
  useEffect(() => {
    setVerified(false);
  }, [transfer?.request_id]);
  if (!native) return null;
  return (
    <section className="settings-card">
      <h2>
        <Wifi size={19} />
        {state?.platform === 'android' ? 'Nearby on Wi-Fi' : 'Nearby Android apps'}
      </h2>
      <p>
        {state?.platform === 'android'
          ? 'While this app is open, desktop installations on the same Wi-Fi can discover it. Approve a desktop import here after checking the matching code. Records are sent only after your approval.'
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
              <button
                className="secondary"
                disabled={
                  disabled ||
                  working ||
                  snapshot.user.role !== 'owner' ||
                  ['waiting', 'verify'].includes(transfer?.status || '')
                }
                onClick={async () => {
                  setWorking(true);
                  setError('');
                  setVerified(false);
                  setIncoming(null);
                  try {
                    setTransfer(await command('request_phone_import', { id: peer.id }));
                  } catch (e) {
                    setError(String(e));
                  } finally {
                    setWorking(false);
                  }
                }}
              >
                Import from phone
              </button>
            </div>
          ))}
          <button className="secondary" onClick={() => void scan()}>
            <RefreshCw size={16} />
            Check again
          </button>
        </>
      )}
      {transfer?.status === 'waiting' && (
        <p role="status">Waiting for your phone. Open Settings there.</p>
      )}
      {['verify', 'approval'].includes(transfer?.status || '') && (
        <div className="transfer-review">
          <h3>Verify both devices</h3>
          <p>Both screens must show this same code:</p>
          <strong className="pairing-code">{transfer?.code}</strong>
          <p>Other device: {transfer?.address}. Only approve a request you started.</p>
          {state?.platform === 'android' && (
            <>
              <label className="checkbox">
                <input
                  type="checkbox"
                  checked={verified}
                  onChange={(e) => setVerified(e.target.checked)}
                />
                The desktop shows the same code
              </label>
              <button
                className="primary"
                disabled={!verified || working || snapshot.user.role !== 'owner'}
                onClick={async () => {
                  setWorking(true);
                  setError('');
                  try {
                    setTransfer(
                      await command('approve_phone_export', {
                        id: transfer?.request_id,
                        contents: JSON.stringify({
                          exported_at: new Date().toISOString(),
                          garage: snapshot.garage,
                        }),
                      }),
                    );
                  } catch (e) {
                    setError(String(e));
                  } finally {
                    setWorking(false);
                  }
                }}
              >
                Approve and send garage
              </button>
              <button
                className="secondary"
                onClick={async () => {
                  await command('reject_phone_export');
                  setTransfer(null);
                  setVerified(false);
                }}
              >
                Decline
              </button>
            </>
          )}
        </div>
      )}
      {transfer?.error && (
        <p className="form-error" role="alert">
          {transfer.error}
        </p>
      )}
      {transfer?.status === 'sent' && (
        <p role="status">Garage sent. Review the import on your desktop.</p>
      )}
      {incoming && (
        <>
          <label className="checkbox">
            <input
              type="checkbox"
              checked={verified}
              onChange={(e) => setVerified(e.target.checked)}
            />
            I checked the matching code on the phone
          </label>
          {verified && (
            <GarageImport
              snapshot={snapshot}
              disabled={disabled}
              onImport={onImport}
              incoming={incoming}
            />
          )}
        </>
      )}
      <small className="settings-help">
        Transfers are encrypted and require phone approval. No web server or account is needed. Open
        Settings on both devices during transfer.
      </small>
    </section>
  );
}
