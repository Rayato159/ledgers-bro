use crate::{Dashboard, credit_cards, receivable_summary, recurring_progress};
use ledger_domain::{DomainError, EntryDate, Money};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AlertPreferences {
    pub updates: bool,
    pub bills: bool,
    pub receivables: bool,
    pub system_enabled: bool,
    pub read: Vec<String>,
    pub notified: Vec<String>,
}
impl Default for AlertPreferences {
    fn default() -> Self {
        Self {
            updates: true,
            bills: true,
            receivables: true,
            system_enabled: false,
            read: Vec::new(),
            notified: Vec::new(),
        }
    }
}
impl AlertPreferences {
    pub fn mark_read(&mut self, key: &str) {
        remember(&mut self.read, key);
    }
    pub fn mark_notified(&mut self, key: &str) {
        remember(&mut self.notified, key);
    }
    pub fn valid(&self) -> bool {
        [&self.read, &self.notified].iter().all(|keys| {
            keys.len() <= 2048
                && keys
                    .iter()
                    .all(|k| !k.is_empty() && k.len() <= 120 && !k.chars().any(char::is_control))
        })
    }
}
fn remember(keys: &mut Vec<String>, key: &str) {
    if keys.iter().any(|k| k == key) {
        return;
    }
    if keys.len() >= 2048 {
        keys.remove(0);
    }
    keys.push(key.into());
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertKind {
    Update,
    Bill,
    Receivable,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerAlert {
    pub key: String,
    pub kind: AlertKind,
    pub title: String,
    pub due: Option<EntryDate>,
    pub amount: Option<Money>,
}

/// Notify unpaid bills up to three days ahead and receivables on their actual
/// collection date. No date means no invented alert.
pub fn ledger_alerts(
    view: &Dashboard,
    prefs: &AlertPreferences,
) -> Result<Vec<LedgerAlert>, DomainError> {
    let mut alerts = Vec::new();
    let soon = |date: EntryDate| (date.date() - view.today.date()).num_days() <= 3;
    if prefs.bills {
        for item in recurring_progress(view) {
            if let Some(month) = item.next_unpaid {
                let due = month.on_day(item.schedule.due().day())?;
                if soon(due) {
                    alerts.push(LedgerAlert {
                        key: format!("bill:{}:{month}", item.schedule.id()),
                        kind: AlertKind::Bill,
                        title: item.schedule.name().as_str().into(),
                        due: Some(due),
                        amount: Some(item.schedule.amount().money()),
                    });
                }
            }
        }
        for card in credit_cards(view)? {
            for bill in card.bills {
                if let Some(dates) = bill.dates
                    && bill.outstanding > Money::ZERO
                    && soon(dates.due)
                {
                    alerts.push(LedgerAlert {
                        key: format!("credit:{}:{}", card.account.id(), dates.due),
                        kind: AlertKind::Bill,
                        title: card.account.name().as_str().into(),
                        due: Some(dates.due),
                        amount: Some(bill.outstanding),
                    });
                }
            }
        }
    }
    if prefs.receivables {
        for item in receivable_summary(view)?.items {
            if item.outstanding > Money::ZERO
                && let Some(due) = item.next_due
                && due <= view.today
            {
                alerts.push(LedgerAlert {
                    key: format!("collect:{}:{due}", item.loan.id()),
                    kind: AlertKind::Receivable,
                    title: format!(
                        "{} · {}",
                        item.loan.debtor().as_str(),
                        item.loan.description().as_str()
                    ),
                    due: Some(due),
                    amount: item.next_amount.or(Some(item.outstanding)),
                });
            }
        }
    }
    alerts.sort_by(|a, b| a.due.cmp(&b.due).then(a.key.cmp(&b.key)));
    Ok(alerts)
}
