use serde_json::{json, Value};
fn patch(base: &mut Value, incoming: &Value, allowed: &[&str]) -> Result<(), String> {
    let fields = incoming.as_object().ok_or("Invalid record patch")?;
    for (key, value) in fields {
        if allowed.contains(&key.as_str()) {
            base[key] = value.clone();
        } else if key != "id" {
            return Err(format!("AI tried to change unsupported field: {key}"));
        }
    }
    Ok(())
}
pub fn normalize(actions: &Value, snapshot: &Value) -> Result<Value, String> {
    let list = actions
        .as_array()
        .ok_or("AI response has no actions array")?;
    if list.len() > 32 {
        return Err("AI returned too many changes".into());
    }
    let mut cars = snapshot["garage"]["cars"]
        .as_array()
        .ok_or("Invalid garage context")?
        .clone();
    let mut result = vec![];
    for input in list {
        let mut action = input.clone();
        let kind = input["type"]
            .as_str()
            .ok_or("AI returned an invalid action")?;
        if kind == "save_car" {
            let id = input["car"]["id"].as_str().unwrap_or("");
            let mut car = if id.is_empty() {
                json!({"id":"","name":"","year":"","make":"","model":"","color":"#72806a","notes":"","cover":"","acquired_from":"","purchase_price_cents":null,"acquired_on":"","items":[],"journal":[]})
            } else {
                cars.iter()
                    .find(|c| c["id"] == id)
                    .ok_or("AI referred to a missing car")?
                    .clone()
            };
            patch(
                &mut car,
                &input["car"],
                &[
                    "name",
                    "year",
                    "make",
                    "model",
                    "color",
                    "notes",
                    "acquired_from",
                    "purchase_price_cents",
                    "acquired_on",
                ],
            )?;
            action["car"] = car;
        } else if kind == "save_item" {
            let car = cars
                .iter()
                .find(|c| c["id"] == input["car_id"])
                .ok_or("AI referred to a missing car")?;
            let id = input["item"]["id"].as_str().unwrap_or("");
            let mut item = if id.is_empty() {
                json!({"id":"","name":"","kind":"part","status":"needed","required":true,"pending":false,"price_cents":0,"seller":"","purchased_on":"","warranty":"","notes":"","receipt":"","receipt_name":""})
            } else {
                car["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|i| i["id"] == id)
                    .ok_or("AI referred to a missing part")?
                    .clone()
            };
            patch(
                &mut item,
                &input["item"],
                &[
                    "name",
                    "kind",
                    "status",
                    "required",
                    "price_cents",
                    "seller",
                    "purchased_on",
                    "warranty",
                    "notes",
                ],
            )?;
            action["item"] = item;
        } else if kind == "add_entry" {
            let mut entry =
                json!({"id":"","date":"","title":"","notes":"","photos":[],"author":""});
            patch(&mut entry, &input["entry"], &["date", "title", "notes"])?;
            action["entry"] = entry;
        } else if ![
            "delete_car",
            "delete_item",
            "set_status",
            "approve_item",
            "delete_entry",
            "configure",
        ]
        .contains(&kind)
        {
            return Err("AI returned an unsupported action".into());
        }
        match kind {
            "save_car" => {
                if let Some(car) = cars.iter_mut().find(|c| c["id"] == action["car"]["id"]) {
                    *car = action["car"].clone();
                }
            }
            "save_item" => {
                if let Some(car) = cars.iter_mut().find(|c| c["id"] == action["car_id"]) {
                    if let Some(item) = car["items"].as_array_mut().and_then(|items| {
                        items.iter_mut().find(|i| i["id"] == action["item"]["id"])
                    }) {
                        *item = action["item"].clone();
                    }
                }
            }
            "set_status" => {
                if let Some(car) = cars.iter_mut().find(|c| c["id"] == action["car_id"]) {
                    if let Some(item) = car["items"]
                        .as_array_mut()
                        .and_then(|items| items.iter_mut().find(|i| i["id"] == action["item_id"]))
                    {
                        item["status"] = action["status"].clone();
                    }
                }
            }
            "delete_car" => cars.retain(|c| c["id"] != action["car_id"]),
            "delete_item" => {
                if let Some(car) = cars.iter_mut().find(|c| c["id"] == action["car_id"]) {
                    if let Some(items) = car["items"].as_array_mut() {
                        items.retain(|i| i["id"] != action["item_id"]);
                    }
                }
            }
            _ => {}
        }
        result.push(action);
    }
    Ok(Value::Array(result))
}
