# Agent changes protocol

A changes file is `{ "protocol": "projectgarage-agent/1", "revision": 12, "actions": [...] }`. Revision must match the snapshot you read. Use 1–32 actions in a batch; do not nest batches. The signed-in user's permissions are enforced. Empty action lists are not a write operation: answer questions from the snapshot instead.

Read-only snapshots include `garage` (name, currency, cars), `revision`, and `user` (name, role). Each car includes its ID, details, acquisition fields, items, and journal. Cover images, receipt binaries, journal photos/authors, and member contact fields are omitted. Snapshot data is not a full replacement record.

## Partial record actions

`save_car`: `{ "type": "save_car", "car": { "id": "existing-id-or-empty", "name": "E30", "acquired_from": "Seller", "purchase_price_cents": 250000 } }`. Optional fields: year, make, model, color, notes, acquired_from, purchase_price_cents (null when unknown), acquired_on (YYYY-MM-DD). Owner only. A new car requires a name. Existing omitted fields are preserved.

`save_item`: `{ "type": "save_item", "car_id": "car-id", "item": { "id": "existing-id-or-empty", "name": "Front brakes", "kind": "part", "status": "purchased", "price_cents": 24995, "seller": "Parts store", "purchased_on": "2026-10-12", "warranty": "12 months" } }`. Optional fields: name, kind (part/job), status, required (boolean), price_cents, seller, purchased_on, warranty, notes. New items require a name. New staff items become pending suggestions. Do not set pending, receipt, receipt_name, photos, or author. Job status is needed/installed and job price is always zero.

`set_status`: `{ "type": "set_status", "car_id": "car-id", "item_id": "item-id", "status": "installed" }`.

`approve_item`: `{ "type": "approve_item", "car_id": "car-id", "item_id": "item-id", "approve": true }`. Owner only. False rejects the pending suggestion.

`add_entry`: `{ "type": "add_entry", "car_id": "car-id", "entry": { "date": "2026-10-12", "title": "Brakes fitted", "notes": "User-provided progress notes" } }`. Adds a text journal entry. Photo attachments remain a normal in-app action.

`configure`: `{ "type": "configure", "name": "Kevin's Project Garage", "currency": "USD" }`. Owner only. Supported currencies: USD, CAD, GBP, EUR, AUD. Changing currency only changes labels; it does not convert existing amounts.

## Deletions

Only when requested: `delete_car` with car_id; `delete_item` with car_id and item_id; `delete_entry` with car_id and entry_id. Owner rules and journal authorship rules are enforced. Removing a car removes its parts/journal; clearly scope the requested car before using this action. Do not delete/recreate an existing record as a shortcut for an edit.

## Example

```json
{
  "protocol": "projectgarage-agent/1",
  "revision": 12,
  "actions": [
    {"type":"set_status","car_id":"actual-car-id","item_id":"actual-part-id","status":"installed"},
    {"type":"add_entry","car_id":"actual-car-id","entry":{"date":"2026-10-12","title":"Front brakes installed","notes":"The user confirmed installation today."}}
  ]
}
```

Use actual IDs and the user's requested dates. Purchased/received/installed approved parts count toward spending; Needed items and pending suggestions do not. Initial car purchase cost also counts. Completion comes from all approved required items reaching Installed.
