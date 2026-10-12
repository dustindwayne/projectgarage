import { useState, useEffect } from 'react';
import type { Snapshot } from './types';
export function GarageImport({
  snapshot,
  disabled,
  onImport,
  incoming,
}: {
  snapshot: Snapshot;
  disabled: boolean;
  onImport: (action: unknown) => Promise<void>;
  incoming?: Snapshot['garage'];
}) {
  const [backup, setBackup] = useState<Snapshot['garage'] | null>(null),
    [error, setError] = useState(''),
    [busy, setBusy] = useState(false);
  useEffect(() => {
    if (incoming) setBackup(incoming);
  }, [incoming]);
  if (snapshot.user.role !== 'owner') return null;
  const added = backup?.cars.filter((c) => !snapshot.garage.cars.some((x) => x.id === c.id)) || [];
  const newItems =
    backup?.cars.reduce(
      (sum, c) =>
        sum +
        c.items.filter(
          (i) => !snapshot.garage.cars.find((x) => x.id === c.id)?.items.some((x) => x.id === i.id),
        ).length,
      0,
    ) || 0;
  const newEntries =
    backup?.cars.reduce(
      (sum, c) =>
        sum +
        c.journal.filter(
          (i) =>
            !snapshot.garage.cars.find((x) => x.id === c.id)?.journal.some((x) => x.id === i.id),
        ).length,
      0,
    ) || 0;
  return (
    <section className="settings-card">
      <h2>Import saved garage</h2>
      <p>
        Choose a garage export saved from your phone or another device. New projects keep their
        parts, receipts, and journal photos. Matching projects gain missing items and journal
        entries; existing records keep their current values.
      </p>
      <label className="field">
        Garage backup
        <input
          type="file"
          accept=".json,application/json"
          disabled={disabled || busy}
          onChange={async (e) => {
            setError('');
            setBackup(null);
            try {
              const file = e.target.files?.[0];
              if (!file) return;
              if (file.size > 100000000) throw Error('Backup exceeds 100 MB');
              const value = JSON.parse(await file.text());
              const g = value.garage;
              if (
                !g ||
                typeof g.name !== 'string' ||
                !Array.isArray(g.cars) ||
                g.cars.some(
                  (c: any) =>
                    typeof c.id !== 'string' ||
                    typeof c.name !== 'string' ||
                    !Array.isArray(c.items) ||
                    !Array.isArray(c.journal),
                )
              )
                throw Error('Choose a Project Garage backup');
              if (g.currency !== snapshot.garage.currency)
                throw Error('Backup currency must match your garage currency');
              setBackup(g);
            } catch (e) {
              setError(String(e));
            }
          }}
        />
      </label>
      {error && (
        <p role="alert" className="form-error">
          {error}
        </p>
      )}
      {backup && (
        <>
          {backup.currency !== snapshot.garage.currency && (
            <p role="alert" className="form-error">
              Backup currency must match your garage currency.
            </p>
          )}
          <p>
            {added.length} new projects · {backup.cars.length - added.length} matching projects ·{' '}
            {newItems} items · {newEntries} journal entries
          </p>
          <ul>
            {added.map((c) => (
              <li key={c.id}>{c.name}</li>
            ))}
          </ul>
          <button
            className="primary"
            disabled={
              disabled ||
              busy ||
              !(added.length + newItems + newEntries) ||
              backup.currency !== snapshot.garage.currency
            }
            onClick={async () => {
              setBusy(true);
              setError('');
              try {
                await onImport({ type: 'import_garage', garage: backup });
                setBackup(null);
              } catch (e) {
                setError(String(e));
              } finally {
                setBusy(false);
              }
            }}
          >
            {busy ? 'Importing…' : 'Confirm import'}
          </button>
        </>
      )}
    </section>
  );
}
