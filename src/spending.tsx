import { useState } from 'react';
import { CarFront, Receipt, Package } from 'lucide-react';
import type { Car, Item } from './types';
import { spent, partsSpent } from './types';
export function Spending({
  cars,
  currency,
  onCar,
  onEdit,
  disabled,
}: {
  cars: Car[];
  currency: string;
  onCar: (id: string) => void;
  onEdit: (id: string, item: Item) => void;
  disabled: boolean;
}) {
  const [filter, setFilter] = useState('all');
  const scoped = filter === 'all' ? cars : cars.filter((c) => c.id === filter);
  const money = (n: number) =>
    new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(n / 100);
  const purchases = scoped
    .flatMap((c) =>
      c.items
        .filter((i) => !i.pending && i.kind === 'part' && i.status !== 'needed')
        .map((i) => ({ car: c, item: i })),
    )
    .sort((a, b) => b.item.purchased_on.localeCompare(a.item.purchased_on));
  const total = scoped.reduce((s, c) => s + spent(c), 0),
    parts = scoped.reduce((s, c) => s + partsSpent(c), 0);
  return (
    <>
      <div className="page-heading">
        <div>
          <h1>
            Spending<span className="heading-dot">.</span>
          </h1>
          <p>Your garage totals and every purchase.</p>
        </div>
      </div>
      <label className="spending-filter">
        Show spending for
        <select value={filter} onChange={(e) => setFilter(e.target.value)}>
          <option value="all">All cars</option>
          {cars.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name}
            </option>
          ))}
        </select>
      </label>
      <section className="spending-stats" aria-label="Spending totals">
        <div className="spend-total">
          <Receipt size={20} />
          <span>{filter === 'all' ? 'Total across your garage' : 'Total for this car'}</span>
          <strong>{money(total)}</strong>
        </div>
        <div>
          <CarFront size={18} />
          <span>Car purchases</span>
          <strong>{money(total - parts)}</strong>
        </div>
        <div>
          <Package size={18} />
          <span>Parts purchased</span>
          <strong>{money(parts)}</strong>
        </div>
      </section>
      <h2 className="spending-title">By car</h2>
      <div className="spending-breakdown">
        {scoped.map((c) => (
          <button key={c.id} onClick={() => onCar(c.id)}>
            <span>
              <strong>{c.name}</strong>
              <small>
                {money(c.purchase_price_cents ?? 0)} car · {money(partsSpent(c))} parts
              </small>
            </span>
            <b>{money(spent(c))}</b>
          </button>
        ))}
      </div>
      <h2 className="spending-title">
        Purchase history{' '}
        <small>
          {purchases.length} parts · {purchases.filter((p) => !!p.item.receipt).length} receipts
        </small>
      </h2>
      <div className="purchase-history">
        {purchases.map(({ car, item }) => (
          <button key={item.id} disabled={disabled} onClick={() => onEdit(car.id, item)}>
            <span>
              <strong>{item.name}</strong>
              <small>
                {car.name} · {item.seller || 'Seller not recorded'}
              </small>
              <small>
                {item.purchased_on || 'Date not recorded'}
                {item.receipt ? ' · Receipt saved' : ''}
              </small>
            </span>
            <b>{money(item.price_cents)}</b>
          </button>
        ))}
        {!purchases.length && (
          <p className="muted">Purchased parts will appear here with dates and receipts.</p>
        )}
      </div>
      {!cars.length && <p className="muted">Add your first project to start tracking spending.</p>}
    </>
  );
}
