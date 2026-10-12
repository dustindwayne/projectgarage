export type ReceiptLine = { name: string; price: string; selected: boolean };
export function parseReceipt(text: string) {
  const lines = text
    .split(/\r?\n/)
    .map((s) => s.trim())
    .filter(Boolean);
  const items: ReceiptLine[] = [];
  let total = '',
    seller =
      lines.find((s) => /[a-z]/i.test(s) && !/[0-9]{2}[/:.-][0-9]{2}/.test(s))?.slice(0, 200) || '';
  for (const line of lines) {
    const match = line.match(/^(.*?)\s+[$£€]?\s*([0-9][0-9,]*[.,][0-9]{2})\s*[A-Z]?$/i);
    if (!match) continue;
    const name = match[1].trim(),
      price = match[2].replace(/,(?=\d{3})/g, '').replace(',', '.');
    if (/^(grand\s+)?total\b|amount\s+(due|paid)/i.test(name)) {
      total = price;
      continue;
    }
    if (
      /subtotal|tax|change|cash|visa|mastercard|payment|balance|tender|discount|saving|shipping|delivery|rounding/i.test(
        name,
      ) ||
      !/[a-z]/i.test(name)
    )
      continue;
    if (items.length < 32) items.push({ name: name.slice(0, 160), price, selected: true });
  }
  let date = '';
  const iso = text.match(/\b(20\d{2})[-/]([01]?\d)[-/]([0-3]?\d)\b/);
  if (iso) date = iso[1] + '-' + iso[2].padStart(2, '0') + '-' + iso[3].padStart(2, '0');
  const us = text.match(/\b([01]?\d)\/([0-3]?\d)\/(20\d{2})\b/);
  if (!date && us) date = us[3] + '-' + us[1].padStart(2, '0') + '-' + us[2].padStart(2, '0');
  if (date) {
    const parsed = new Date(date + 'T00:00:00Z');
    if (!Number.isFinite(parsed.getTime()) || parsed.toISOString().slice(0, 10) !== date) date = '';
  }
  return { items, total, seller, date };
}
