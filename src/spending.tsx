import { useState } from 'react';
import { Receipt } from 'lucide-react';
import type { Car, Item } from './types';
import { ReceiptScan } from './receipt-scan';
import { native } from './api';
type Purchase = {
  car: Car;
  name: string;
  price: number;
  date: string;
  seller: string;
  item?: Item;
};
export function Spending({
  cars,
  currency,
  onCar,
  onEdit,
  disabled,
  owner,
  onSave,
}: {
  cars: Car[];
  currency: string;
  onCar: (id: string) => void;
  onEdit: (id: string, item: Item) => void;
  disabled: boolean;
  owner: boolean;
  onSave: (action: unknown) => Promise<void>;
}) {
  const [filter, setFilter] = useState('all'),
    [from, setFrom] = useState(''),
    [until, setUntil] = useState(''),
    [search, setSearch] = useState(''),
    [sort, setSort] = useState('newest'),
    [exportMessage, setExportMessage] = useState('');
  const money = (n: number) =>
    new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(n / 100);
  const scoped = filter === 'all' ? cars : cars.filter((c) => c.id === filter);
  const all: Purchase[] = scoped.flatMap((car) => [
    ...(car.purchase_price_cents !== null && car.purchase_price_cents !== undefined
      ? [
          {
            car,
            name: 'Car purchase',
            price: car.purchase_price_cents,
            date: car.acquired_on || '',
            seller: car.acquired_from || '',
          },
        ]
      : []),
    ...car.items
      .filter((i) => !i.pending && i.kind === 'part' && i.status !== 'needed')
      .map((item) => ({
        car,
        name: item.name,
        price: item.price_cents,
        date: item.purchased_on,
        seller: item.seller,
        item,
      })),
  ]);
  const dated = (p: Purchase) => /^\d{4}-\d{2}-\d{2}$/.test(p.date);
  const purchases = all.filter(
    (p) =>
      (!from && !until) || (dated(p) && (!from || p.date >= from) && (!until || p.date <= until)),
  );
  const total = purchases.reduce((s, p) => s + p.price, 0),
    parts = purchases.filter((p) => p.item),
    partTotal = parts.reduce((s, p) => s + p.price, 0),
    largest = [...purchases].sort((a, b) => b.price - a.price)[0];
  const history = purchases
    .filter((p) =>
      (p.name + ' ' + p.seller + ' ' + p.car.name).toLowerCase().includes(search.toLowerCase()),
    )
    .sort((a, b) =>
      sort === 'highest'
        ? b.price - a.price
        : sort === 'lowest'
          ? a.price - b.price
          : b.date.localeCompare(a.date),
    );
  const group = (key: (p: Purchase) => string) => {
    const m = new Map<string, number>();
    for (const p of purchases) m.set(key(p), (m.get(key(p)) || 0) + p.price);
    return [...m].sort((a, b) => b[1] - a[1]);
  };
  const months = group((p) => (dated(p) ? p.date.slice(0, 7) : 'Undated')).sort((a, b) =>
    a[0].localeCompare(b[0]),
  );
  async function exportCsv() {
    const rows = [
      ['Project', 'Item', 'Type', 'Purchase date', 'Seller', 'Price paid', 'Currency', 'Receipt'],
      ...history.map((p) => [
        p.car.name,
        p.name,
        p.item ? 'Part' : 'Car',
        p.date,
        p.seller,
        (p.price / 100).toFixed(2),
        currency,
        p.item?.receipt ? 'Saved' : '',
      ]),
    ];
    const csv = rows
      .map((row) =>
        row
          .map(
            (cell) =>
              '"' + String(/^[=+@-]/.test(cell) ? "'" + cell : cell).replaceAll('"', '""') + '"',
          )
          .join(','),
      )
      .join('\r\n');
    if (native) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const path = await invoke<string>('export_spending_csv', { contents: '\uFEFF' + csv });
        setExportMessage('Purchase export saved: ' + path);
      } catch (e) {
        setExportMessage(String(e));
      }
      return;
    }
    const url = URL.createObjectURL(new Blob(['\uFEFF' + csv], { type: 'text/csv;charset=utf-8' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = 'project-garage-purchases.csv';
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  return (
    <>
      <div className="page-heading">
        <div>
          <h1>
            Spending<span className="heading-dot">.</span>
          </h1>
          <p>Every purchase. A clearer picture of your build.</p>
        </div>
      </div>
      <div className="spending-controls">
        <label className="field">
          Project
          <select value={filter} onChange={(e) => setFilter(e.target.value)}>
            <option value="all">All cars</option>
            {cars.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
        </label>
        <details className="date-filters">
          <summary>
            {from || until ? `Dates: ${from || 'start'} → ${until || 'today'}` : 'Filter by date'}
          </summary>
          <div className="form-grid">
            <label className="field">
              From
              <input
                type="date"
                value={from}
                max={until || undefined}
                onChange={(e) => setFrom(e.target.value)}
              />
            </label>
            <label className="field">
              Through
              <input
                type="date"
                value={until}
                min={from || undefined}
                onChange={(e) => setUntil(e.target.value)}
              />
            </label>
          </div>
          {(from || until) && (
            <button
              className="text-link"
              onClick={() => {
                setFrom('');
                setUntil('');
              }}
            >
              Show all dates
            </button>
          )}
        </details>
      </div>
      <section className="spending-stats" aria-label="Spending totals">
        <div className="spend-total">
          <Receipt size={20} />
          <span>{from || until ? 'Spending in selected dates' : 'Total spending'}</span>
          <strong>{money(total)}</strong>
        </div>
        <div>
          <span>Car purchases</span>
          <strong>{money(total - partTotal)}</strong>
        </div>
        <div>
          <span>Parts purchased</span>
          <strong>{money(partTotal)}</strong>
        </div>
      </section>
      <section className="price-insights" aria-label="Purchase statistics">
        <div>
          <span>Part purchases</span>
          <strong>{parts.length}</strong>
        </div>
        <div>
          <span>Average part price</span>
          <strong>{money(parts.length ? Math.round(partTotal / parts.length) : 0)}</strong>
        </div>
        <div>
          <span>Largest purchase</span>
          <strong>{money(largest?.price || 0)}</strong>
          <small>{largest?.name || 'No purchases yet'}</small>
        </div>
        <div>
          <span>Receipts saved</span>
          <strong>
            {parts.filter((p) => !!p.item?.receipt).length} / {parts.length}
          </strong>
        </div>
      </section>
      {all.some((p) => !dated(p)) && (
        <p className="muted">
          {all.filter((p) => !dated(p)).length} purchases have no recorded date
          {from || until ? ' and are excluded from this date range' : ''}.
        </p>
      )}
      <h2 className="spending-title">Spending by month</h2>
      <div className="monthly-spending">
        {months.map(([month, value]) => (
          <div key={month}>
            <div>
              <span>{month}</span>
              <strong>{money(value)}</strong>
            </div>
            <meter
              min={0}
              max={Math.max(1, ...months.map((m) => m[1]))}
              value={value}
              aria-label={month + ' spending'}
            />
          </div>
        ))}
        {!months.length && <p className="muted">Your purchase history will build this chart.</p>}
      </div>
      <h2 className="spending-title">By car</h2>
      <div className="spending-breakdown">
        {scoped.map((car) => (
          <button key={car.id} onClick={() => onCar(car.id)}>
            <span>
              <strong>{car.name}</strong>
              <small>{purchases.filter((p) => p.car.id === car.id).length} purchases</small>
            </span>
            <b>
              {money(purchases.filter((p) => p.car.id === car.id).reduce((s, p) => s + p.price, 0))}
            </b>
          </button>
        ))}
      </div>
      <details className="seller-stats">
        <summary>Spending by seller</summary>
        {group((p) => p.seller.trim() || 'Seller not recorded').map(([seller, value]) => (
          <div className="stat-row" key={seller}>
            <span>{seller}</span>
            <strong>{money(value)}</strong>
          </div>
        ))}
      </details>
      <h2 className="spending-title">
        Purchase history <small>{purchases.length} purchases</small>
      </h2>
      <label className="field">
        Search purchases
        <input
          type="search"
          placeholder="Part, seller, or project"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </label>
      <label className="field">
        Sort by
        <select value={sort} onChange={(e) => setSort(e.target.value)}>
          <option value="newest">Newest first</option>
          <option value="highest">Highest price first</option>
          <option value="lowest">Lowest price first</option>
        </select>
      </label>
      <button className="secondary" disabled={!history.length} onClick={() => void exportCsv()}>
        Export filtered purchases (CSV)
      </button>
      {exportMessage && <p role="status">{exportMessage}</p>}
      <div className="purchase-history">
        {history.map((p) => (
          <button
            key={p.car.id + ':' + (p.item?.id || 'car')}
            disabled={disabled && !!p.item}
            onClick={() => (p.item ? onEdit(p.car.id, p.item) : onCar(p.car.id))}
          >
            <span>
              <strong>{p.name}</strong>
              <small>
                {p.car.name} · {p.seller || 'Seller not recorded'}
              </small>
              <small>
                {p.date || 'Date not recorded'}
                {p.item?.receipt ? ' · Receipt saved' : ''}
              </small>
            </span>
            <b>{money(p.price)}</b>
          </button>
        ))}
        {!history.length && <p className="muted">No purchases match these filters.</p>}
      </div>
      <ReceiptScan key={filter} cars={scoped} owner={owner} disabled={disabled} onSave={onSave} />
    </>
  );
}
