use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Staff,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Needed,
    Purchased,
    Received,
    Installed,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub status: Status,
    pub required: bool,
    pub pending: bool,
    pub price_cents: i64,
    pub seller: String,
    pub purchased_on: String,
    pub warranty: String,
    pub receipt: String,
    pub receipt_name: String,
    pub notes: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub date: String,
    pub title: String,
    pub notes: String,
    pub photos: Vec<String>,
    pub author: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Car {
    pub id: String,
    pub name: String,
    pub year: String,
    pub make: String,
    pub model: String,
    pub color: String,
    pub notes: String,
    pub cover: String,
    #[serde(default)]
    pub acquired_from: String,
    #[serde(default)]
    pub purchase_price_cents: Option<i64>,
    #[serde(default)]
    pub acquired_on: String,
    pub items: Vec<Item>,
    pub journal: Vec<Entry>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Garage {
    pub name: String,
    pub currency: String,
    pub cars: Vec<Car>,
}
impl Garage {
    pub fn new(name: &str) -> Self {
        Self {
            name: format!("{name}'s Project Garage"),
            currency: "USD".into(),
            cars: vec![],
        }
    }
    pub fn apply(&mut self, role: &Role, author: &str, action: Action) -> Result<(), String> {
        let owner = *role == Role::Owner;
        match action {
            Action::Batch { actions } => {
                if actions.is_empty()
                    || actions.len() > 32
                    || actions.iter().any(|a| matches!(a, Action::Batch { .. }))
                {
                    return Err("Use between 1 and 32 non-nested changes".into());
                }
                let mut next = self.clone();
                for action in actions {
                    next.apply(role, author, action)?;
                }
                *self = next;
            }
            Action::Configure { name, currency } => {
                require_owner(owner)?;
                valid_name(&name)?;
                if !["USD", "CAD", "GBP", "EUR", "AUD"].contains(&currency.as_str()) {
                    return Err("Choose a supported currency".into());
                }
                self.name = name;
                self.currency = currency;
            }
            Action::SaveCar { mut car } => {
                require_owner(owner)?;
                valid_name(&car.name)?;
                validate_image(&car.cover)?;
                if car.acquired_from.len() > 200
                    || car
                        .purchase_price_cents
                        .is_some_and(|price| !(0..=100_000_000).contains(&price))
                {
                    return Err("Enter a valid car source and purchase price".into());
                }
                if let Some(existing) = self.cars.iter_mut().find(|c| c.id == car.id) {
                    car.items = existing.items.clone();
                    car.journal = existing.journal.clone();
                    *existing = car;
                } else {
                    car.id = new_id();
                    car.items.clear();
                    car.journal.clear();
                    self.cars.push(car);
                }
            }
            Action::DeleteCar { car_id } => {
                require_owner(owner)?;
                let index = self
                    .cars
                    .iter()
                    .position(|c| c.id == car_id)
                    .ok_or("Car not found")?;
                self.cars.remove(index);
            }
            Action::SaveItem { car_id, mut item } => {
                valid_name(&item.name)?;
                if !["part", "job"].contains(&item.kind.as_str()) {
                    return Err("Invalid item type".into());
                }
                if item.price_cents < 0 || item.price_cents > 100_000_000 {
                    return Err("Enter a valid price".into());
                }
                validate_receipt(&item.receipt)?;
                if item.kind == "job" {
                    if ![Status::Needed, Status::Installed].contains(&item.status) {
                        return Err("Jobs are either needed or completed".into());
                    }
                    item.price_cents = 0;
                    item.receipt.clear();
                    item.receipt_name.clear();
                }
                let car = self.car_mut(&car_id)?;
                if let Some(existing) = car.items.iter_mut().find(|i| i.id == item.id) {
                    if !owner
                        && (existing.pending
                            || item.required != existing.required
                            || item.pending != existing.pending
                            || item.kind != existing.kind)
                    {
                        return Err("The owner must approve required items".into());
                    }
                    item.pending = existing.pending;
                    *existing = item;
                } else {
                    item.id = new_id();
                    item.pending = !owner;
                    if item.pending {
                        item.status = Status::Needed;
                        item.price_cents = 0;
                    }
                    car.items.push(item);
                }
            }
            Action::ApproveItem {
                car_id,
                item_id,
                approve,
            } => {
                require_owner(owner)?;
                let car = self.car_mut(&car_id)?;
                let index = car
                    .items
                    .iter()
                    .position(|i| i.id == item_id && i.pending)
                    .ok_or("Suggestion not found")?;
                if approve {
                    car.items[index].pending = false;
                } else {
                    car.items.remove(index);
                }
            }
            Action::DeleteItem { car_id, item_id } => {
                require_owner(owner)?;
                let car = self.car_mut(&car_id)?;
                let index = car
                    .items
                    .iter()
                    .position(|i| i.id == item_id)
                    .ok_or("Item not found")?;
                car.items.remove(index);
            }
            Action::SetStatus {
                car_id,
                item_id,
                status,
            } => {
                let item = self
                    .car_mut(&car_id)?
                    .items
                    .iter_mut()
                    .find(|i| i.id == item_id)
                    .ok_or("Item not found")?;
                if item.pending {
                    return Err("This suggestion needs owner approval first".into());
                }
                if item.kind == "job" && ![Status::Needed, Status::Installed].contains(&status) {
                    return Err("Jobs are either needed or completed".into());
                }
                item.status = status;
            }
            Action::AddEntry { car_id, mut entry } => {
                valid_name(&entry.title)?;
                if entry.date.len() != 10 {
                    return Err("Choose a journal date".into());
                }
                if entry.photos.len() > 6 {
                    return Err("Use up to six photos per entry".into());
                }
                for photo in &entry.photos {
                    validate_image(photo)?;
                }
                entry.id = new_id();
                entry.author = author.into();
                self.car_mut(&car_id)?.journal.push(entry);
            }
            Action::DeleteEntry { car_id, entry_id } => {
                let car = self.car_mut(&car_id)?;
                let index = car
                    .journal
                    .iter()
                    .position(|e| e.id == entry_id)
                    .ok_or("Entry not found")?;
                if !owner && car.journal[index].author != author {
                    return Err("Only the author or owner can remove this entry".into());
                }
                car.journal.remove(index);
            }
        }
        Ok(())
    }
    fn car_mut(&mut self, id: &str) -> Result<&mut Car, String> {
        self.cars
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or("Car not found".into())
    }
}
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn valid_name(s: &str) -> Result<(), String> {
    if s.trim().is_empty() || s.len() > 160 {
        Err("Enter a name between 1 and 160 characters".into())
    } else {
        Ok(())
    }
}
fn require_owner(owner: bool) -> Result<(), String> {
    if owner {
        Ok(())
    } else {
        Err("Only the garage owner can do this".into())
    }
}
fn validate_image(s: &str) -> Result<(), String> {
    if s.is_empty()
        || (s.len() <= 3_000_000
            && [
                "data:image/jpeg;base64,",
                "data:image/png;base64,",
                "data:image/webp;base64,",
            ]
            .iter()
            .any(|p| s.starts_with(p)))
    {
        Ok(())
    } else {
        Err("Use a JPEG, PNG or WebP image under 2 MB".into())
    }
}
fn validate_receipt(s: &str) -> Result<(), String> {
    if s.starts_with("data:application/pdf;base64,") && s.len() <= 4_000_000 {
        Ok(())
    } else {
        validate_image(s)
    }
}
pub fn spending(car: &Car) -> i64 {
    car.purchase_price_cents.unwrap_or(0) + parts_spending(car)
}
pub fn parts_spending(car: &Car) -> i64 {
    car.items
        .iter()
        .filter(|i| !i.pending && i.kind == "part" && i.status != Status::Needed)
        .map(|i| i.price_cents)
        .sum()
}
pub fn progress(car: &Car) -> (usize, usize) {
    let required: Vec<_> = car
        .items
        .iter()
        .filter(|i| i.required && !i.pending)
        .collect();
    (
        required
            .iter()
            .filter(|i| i.status == Status::Installed)
            .count(),
        required.len(),
    )
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Batch {
        actions: Vec<Action>,
    },
    Configure {
        name: String,
        currency: String,
    },
    SaveCar {
        car: Car,
    },
    DeleteCar {
        car_id: String,
    },
    SaveItem {
        car_id: String,
        item: Item,
    },
    ApproveItem {
        car_id: String,
        item_id: String,
        approve: bool,
    },
    DeleteItem {
        car_id: String,
        item_id: String,
    },
    SetStatus {
        car_id: String,
        item_id: String,
        status: Status,
    },
    AddEntry {
        car_id: String,
        entry: Entry,
    },
    DeleteEntry {
        car_id: String,
        entry_id: String,
    },
}
#[cfg(test)]
mod tests {
    use super::*;
    fn car() -> Car {
        serde_json::from_value(serde_json::json!({"id":"", "name":"E30", "year":"1989", "make":"BMW", "model":"325i", "color":"#637968", "notes":"", "cover":"", "items":[], "journal":[]})).unwrap()
    }
    fn item() -> Item {
        serde_json::from_value(serde_json::json!({"id":"", "name":"Brakes", "kind":"part", "status":"needed", "required":true,"pending":false,"price_cents":12500,"seller":"", "purchased_on":"", "warranty":"", "receipt":"", "receipt_name":"", "notes":""})).unwrap()
    }
    #[test]
    fn batch_rolls_back_on_invalid_action_and_enforces_role() {
        let mut g = Garage::new("Kevin");
        let batch = || Action::Batch {
            actions: vec![
                Action::Configure {
                    name: "Changed".into(),
                    currency: "USD".into(),
                },
                Action::DeleteCar {
                    car_id: "missing".into(),
                },
            ],
        };
        assert!(g.apply(&Role::Owner, "Kevin", batch()).is_err());
        assert_eq!(g.name, "Kevin's Project Garage");
        assert!(g.apply(&Role::Staff, "Staff", batch()).is_err());
        assert!(g
            .apply(
                &Role::Owner,
                "Kevin",
                Action::Batch {
                    actions: vec![Action::Batch { actions: vec![] }]
                }
            )
            .is_err());
    }
    #[test]
    fn staff_cannot_create_or_configure_cars() {
        let mut g = Garage::new("Kevin");
        assert!(g
            .apply(&Role::Staff, "Staff", Action::SaveCar { car: car() })
            .is_err());
        assert!(g
            .apply(
                &Role::Staff,
                "Staff",
                Action::Configure {
                    name: "Other".into(),
                    currency: "USD".into()
                }
            )
            .is_err());
    }
    #[test]
    fn approval_gates_progress_and_spending() {
        let mut g = Garage::new("Kevin");
        g.apply(&Role::Owner, "Kevin", Action::SaveCar { car: car() })
            .unwrap();
        let cid = g.cars[0].id.clone();
        g.apply(
            &Role::Staff,
            "Staff",
            Action::SaveItem {
                car_id: cid.clone(),
                item: item(),
            },
        )
        .unwrap();
        let iid = g.cars[0].items[0].id.clone();
        assert!(g.cars[0].items[0].pending);
        assert_eq!(progress(&g.cars[0]), (0, 0));
        assert_eq!(spending(&g.cars[0]), 0);
        assert!(g
            .apply(
                &Role::Staff,
                "Staff",
                Action::SetStatus {
                    car_id: cid.clone(),
                    item_id: iid.clone(),
                    status: Status::Installed
                }
            )
            .is_err());
        g.apply(
            &Role::Owner,
            "Kevin",
            Action::ApproveItem {
                car_id: cid.clone(),
                item_id: iid.clone(),
                approve: true,
            },
        )
        .unwrap();
        assert_eq!(progress(&g.cars[0]), (0, 1));
        g.apply(
            &Role::Staff,
            "Staff",
            Action::SetStatus {
                car_id: cid,
                item_id: iid,
                status: Status::Installed,
            },
        )
        .unwrap();
        assert_eq!(progress(&g.cars[0]), (1, 1));
    }
    #[test]
    fn needed_prices_do_not_count_as_spent() {
        let mut c = car();
        c.items.push(item());
        assert_eq!(spending(&c), 0);
        c.items[0].status = Status::Purchased;
        assert_eq!(spending(&c), 12500);
    }
    #[test]
    fn staff_cannot_remove_required_item() {
        let mut g = Garage::new("Kevin");
        g.apply(&Role::Owner, "Kevin", Action::SaveCar { car: car() })
            .unwrap();
        let cid = g.cars[0].id.clone();
        g.apply(
            &Role::Owner,
            "Kevin",
            Action::SaveItem {
                car_id: cid.clone(),
                item: item(),
            },
        )
        .unwrap();
        let mut changed = g.cars[0].items[0].clone();
        changed.required = false;
        assert!(g
            .apply(
                &Role::Staff,
                "Staff",
                Action::SaveItem {
                    car_id: cid,
                    item: changed
                }
            )
            .is_err());
    }
    #[test]
    fn acquisition_defaults_preserve_old_cars_and_total_includes_purchase() {
        let mut c = car();
        assert_eq!(c.purchase_price_cents, None);
        assert!(c.acquired_from.is_empty());
        c.purchase_price_cents = Some(120000);
        let mut part = item();
        part.status = Status::Purchased;
        c.items.push(part);
        assert_eq!(parts_spending(&c), 12500);
        assert_eq!(spending(&c), 132500);
        c.purchase_price_cents = Some(0);
        assert_eq!(spending(&c), 12500);
    }
    #[test]
    fn jobs_cannot_be_purchased_or_add_labor_costs() {
        let mut g = Garage::new("Kevin");
        g.apply(&Role::Owner, "Kevin", Action::SaveCar { car: car() })
            .unwrap();
        let cid = g.cars[0].id.clone();
        let mut work = item();
        work.kind = "job".into();
        work.status = Status::Purchased;
        assert!(g
            .apply(
                &Role::Owner,
                "Kevin",
                Action::SaveItem {
                    car_id: cid.clone(),
                    item: work.clone()
                }
            )
            .is_err());
        work.status = Status::Installed;
        g.apply(
            &Role::Owner,
            "Kevin",
            Action::SaveItem {
                car_id: cid,
                item: work,
            },
        )
        .unwrap();
        assert_eq!(g.cars[0].items[0].price_cents, 0);
        assert_eq!(progress(&g.cars[0]), (1, 1));
        assert_eq!(spending(&g.cars[0]), 0);
    }
}
