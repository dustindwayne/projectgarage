import fs from 'node:fs';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import ts from 'typescript';
const js = ts.transpileModule(fs.readFileSync('src/receipt-parser.ts', 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS },
}).outputText;
const sandbox = { exports: {} };
vm.runInNewContext(js, sandbox);
const parse = sandbox.exports.parseReceipt;
const result = parse(
  'GARAGE PARTS STORE\n10/12/2026\nBrake Kit 249.95\nOil Filter 12.50\nSubtotal 262.45\nTax 21.00\nTOTAL 283.45\nVisa 283.45',
);
assert.equal(result.items.length, 2);
assert.equal(result.items[0].name, 'Brake Kit');
assert.equal(result.items[1].price, '12.50');
assert.equal(result.total, '283.45');
assert.equal(result.date, '2026-10-12');
assert.equal(parse('Parts 1,249.99\n2026-02-30').date, '');
assert.equal(parse('Parts 1,249.99').items[0].price, '1249.99');
assert.equal(parse('2026/19/10').date, '');
assert.equal(parse('No readable prices').items.length, 0);
assert.equal(parse('Oil Filter 12,50').items[0].price, '12.50');
console.log(
  'Receipt extraction: line prices, excluded totals/tax/payment, dates, malformed dates, and decimal formats passed.',
);
