import { useState } from 'react';
import { createWorker } from 'tesseract.js';
import { receiptFile } from './api';
import { emptyItem } from './types';
import type { Car } from './types';
import { parseReceipt, type ReceiptLine } from './receipt-parser';
export function ReceiptScan({
  cars,
  carId,
  owner,
  disabled,
  onSave,
}: {
  cars: Car[];
  carId?: string;
  owner: boolean;
  disabled: boolean;
  onSave: (action: unknown) => Promise<void>;
}) {
  const [open, setOpen] = useState(false),
    [target, setTarget] = useState(carId || cars[0]?.id || ''),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(''),
    [progress, setProgress] = useState(''),
    [image, setImage] = useState(''),
    [fileName, setFileName] = useState(''),
    [text, setText] = useState(''),
    [items, setItems] = useState<ReceiptLine[]>([]),
    [seller, setSeller] = useState(''),
    [date, setDate] = useState(''),
    [total, setTotal] = useState('');
  const sum = items
    .filter((i) => i.selected)
    .reduce((s, i) => s + Math.round(Number(i.price) * 100), 0);
  async function scan(file: File) {
    setBusy(true);
    setError('');
    setItems([]);
    setText('');
    let worker;
    try {
      const receipt = await receiptFile(file);
      setImage(receipt);
      setFileName(file.name);
      worker = await createWorker('eng', 1, {
        workerPath: new URL('/ocr/worker.min.js', location.href).href,
        corePath: new URL('/ocr/', location.href).href,
        langPath: new URL('/ocr', location.href).href,
        workerBlobURL: true,
        logger: (m) =>
          setProgress(m.status + (m.progress ? ' ' + Math.round(m.progress * 100) + '%' : '')),
      });
      const result = await worker.recognize(file);
      const parsed = parseReceipt(result.data.text);
      setText(result.data.text);
      setItems(parsed.items);
      setSeller(parsed.seller);
      setDate(parsed.date);
      setTotal(parsed.total);
    } catch (e) {
      setError('Could not read this receipt: ' + String(e));
    } finally {
      await worker?.terminate();
      setBusy(false);
    }
  }
  if (!open)
    return (
      <button
        className="secondary"
        disabled={disabled || !cars.length}
        onClick={() => setOpen(true)}
      >
        Scan receipt image
      </button>
    );
  return (
    <section className="settings-card receipt-scan">
      <h2>Review receipt</h2>
      <p>
        Read on this device. Check every field before saving. Prices are line totals; tax, shipping,
        and discounts are not added automatically.
      </p>
      <label className="field">
        Project
        <select value={target} disabled={busy} onChange={(e) => setTarget(e.target.value)}>
          {cars.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name}
            </option>
          ))}
        </select>
      </label>
      <label className="field">
        Receipt image
        <input
          type="file"
          accept="image/jpeg,image/png,image/webp"
          disabled={busy}
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) void scan(f);
          }}
        />
      </label>
      {busy && <p role="status">{progress || 'Reading receipt…'}</p>}
      {error && (
        <p role="alert" className="form-error">
          {error}
        </p>
      )}
      {text && (
        <>
          <img className="scan-preview" src={image} alt="Receipt being reviewed" />
          <label className="field">
            Seller
            <input value={seller} maxLength={200} onChange={(e) => setSeller(e.target.value)} />
          </label>
          <label className="field">
            Purchase date
            <input type="date" value={date} onChange={(e) => setDate(e.target.value)} />
          </label>
          {items.map((item, index) => (
            <div className="scan-line" key={index}>
              <label className="checkbox">
                <input
                  type="checkbox"
                  checked={item.selected}
                  onChange={(e) =>
                    setItems(
                      items.map((i, n) => (n === index ? { ...i, selected: e.target.checked } : i)),
                    )
                  }
                />
                Include
              </label>
              <label className="field">
                Item
                <input
                  value={item.name}
                  maxLength={160}
                  onChange={(e) =>
                    setItems(
                      items.map((i, n) => (n === index ? { ...i, name: e.target.value } : i)),
                    )
                  }
                />
              </label>
              <label className="field">
                Line price
                <input
                  type="number"
                  min="0"
                  max="1000000"
                  step="0.01"
                  value={item.price}
                  onChange={(e) =>
                    setItems(
                      items.map((i, n) => (n === index ? { ...i, price: e.target.value } : i)),
                    )
                  }
                />
              </label>
            </div>
          ))}
          <button
            className="secondary"
            onClick={() => setItems([...items, { name: '', price: '0.00', selected: true }])}
            disabled={items.length >= 32}
          >
            Add missing item
          </button>
          <p>
            Selected items: {(sum / 100).toFixed(2)}
            {total && ' · Receipt total: ' + total}
          </p>
          <p className="muted">
            If these differ, review quantities, discounts, tax and shipping. No receipt total is
            added as an extra purchase.
          </p>
          <details>
            <summary>Recognized text</summary>
            <pre className="ocr-text">{text}</pre>
          </details>
          {!owner && (
            <p>
              These will be suggestions pending owner approval. The reviewed price remains in each
              suggestion’s notes.
            </p>
          )}
          <button
            className="primary"
            disabled={
              busy ||
              disabled ||
              !items.some((i) => i.selected) ||
              items.some(
                (i) =>
                  i.selected &&
                  (!i.name.trim() ||
                    !i.price ||
                    !Number.isFinite(Number(i.price)) ||
                    Number(i.price) < 0 ||
                    Number(i.price) > 1000000),
              )
            }
            onClick={async () => {
              setBusy(true);
              setError('');
              try {
                await onSave({
                  type: 'batch',
                  actions: items
                    .filter((i) => i.selected)
                    .map((i) => ({
                      type: 'save_item',
                      car_id: target,
                      item: {
                        ...emptyItem(),
                        name: i.name,
                        required: false,
                        status: 'purchased',
                        price_cents: Math.round(Number(i.price) * 100),
                        seller,
                        purchased_on: date,
                        receipt: image,
                        receipt_name: fileName,
                        notes: owner
                          ? 'Imported from reviewed receipt'
                          : 'Reviewed receipt price: ' + i.price,
                      },
                    })),
                });
                setOpen(false);
                setItems([]);
                setText('');
              } catch (e) {
                setError(String(e));
              } finally {
                setBusy(false);
              }
            }}
          >
            Confirm and add selected items
          </button>
        </>
      )}
      <button
        className="text-link"
        disabled={busy}
        onClick={() => {
          setOpen(false);
          setText('');
          setItems([]);
        }}
      >
        Cancel
      </button>
    </section>
  );
}
