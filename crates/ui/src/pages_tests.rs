#![allow(clippy::expect_used)]
use super::*;
use crate::{Gateway, UiFuture, UiGateway};
use dioxus::dioxus_core::{ElementId, Mutation, Mutations};
use dioxus::html::{PlatformEventData, SerializedFormData, set_event_converter};
use ledger_application::{
    AppError, CommitOutcome, CsvExport, LedgerState, ReceiptImage, Response, dashboard,
};
use std::{
    any::Any,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

fn entry(id: u32, date: &str, income: bool) -> JournalEntry {
    let account = "00000000-0000-0000-0000-000000000001"
        .parse()
        .expect("account");
    let amount = PositiveMoney::new("1.00".parse().expect("money")).expect("positive");
    JournalEntry::record(
        format!("00000000-0000-0000-0000-{id:012}")
            .parse()
            .expect("id"),
        date.parse().expect("date"),
        Note::new("Synthetic history entry").expect("note"),
        if income {
            EntryKind::Income {
                account,
                amount,
                category: Category::OtherIncome,
            }
        } else {
            EntryKind::Expense {
                account,
                amount,
                category: Category::OtherExpense,
            }
        },
    )
    .expect("entry")
}

#[test]
fn history_sorts_dates_before_paging_and_preserves_same_day_order() {
    let newest_inserted = entry(1, "2026-10-03", false);
    let fourth = entry(2, "2026-10-04", false);
    let another_fourth = entry(3, "2026-10-04", true);
    let input = vec![
        newest_inserted.clone(),
        fourth.clone(),
        another_fourth.clone(),
        entry(4, "2025-12-31", false),
    ];
    let result = history_entries(&input, 0, 0, None, None, None);
    assert_eq!(&result[..3], &[fourth, another_fourth, newest_inserted]);
    assert_eq!(
        input[0].date().to_string(),
        "2026-10-03",
        "source journal order is unchanged"
    );
    let many: Vec<_> = (1..=31)
        .map(|i| entry(i, &format!("2026-10-{i:02}"), false))
        .collect();
    let sorted = history_entries(&many, 0, 0, None, None, None);
    assert_eq!(sorted[0].date().to_string(), "2026-10-31");
    assert_eq!(sorted[20].date().to_string(), "2026-10-11");
}

#[test]
fn history_filters_inclusive_day_range_year_and_type() {
    let entries = vec![
        entry(1, "2025-12-31", false),
        entry(2, "2026-01-01", false),
        entry(3, "2026-01-01", true),
        entry(4, "2026-01-02", false),
    ];
    let (from, through) = history_dates("2025-12-31", "2026-01-01").expect("cross-year range");
    assert_eq!(
        history_entries(&entries, 0, 0, from, through, None).len(),
        3
    );
    assert_eq!(
        history_entries(&entries, 1, 2026, from, through, None),
        vec![entries[1].clone()]
    );
    let (from, through) = history_dates("2026-01-01", "2026-01-01").expect("single day");
    assert_eq!(
        history_entries(&entries, 2, 0, from, through, None),
        vec![entries[2].clone()]
    );
    assert_eq!(
        history_entries(&entries, 0, 0, None, through, None).len(),
        3
    );
    assert_eq!(history_entries(&entries, 0, 0, from, None, None).len(), 3);
    assert!(history_dates("2026-10-04", "2026-10-03").is_err());
    assert!(history_dates("2026-02-30", "").is_err());
    assert_eq!(history_dates("", ""), Ok((None, None)));
}

#[test]
fn account_history_matches_both_transfer_ends_and_lending_postings() {
    let bank = "00000000-0000-0000-0000-000000000101"
        .parse::<AccountId>()
        .expect("synthetic id");
    let card = "00000000-0000-0000-0000-000000000102"
        .parse::<AccountId>()
        .expect("synthetic id");
    let other = "00000000-0000-0000-0000-000000000103"
        .parse::<AccountId>()
        .expect("synthetic id");
    let receivable = "00000000-0000-0000-0000-000000000105"
        .parse::<ReceivableId>()
        .expect("synthetic id");
    let amount = PositiveMoney::new("25.00".parse().expect("money")).expect("positive");
    let kinds = [
        EntryKind::Transfer {
            from: bank,
            to: card,
            amount,
        },
        EntryKind::Lending {
            receivable,
            account: bank,
            amount,
        },
        EntryKind::Repayment {
            receivable,
            account: bank,
            amount,
        },
        EntryKind::Expense {
            account: other,
            amount,
            category: Category::Food,
        },
        EntryKind::Opening {
            account: bank,
            balance: "100.00".parse().expect("money"),
        },
    ];
    let entries: Vec<_> = kinds
        .into_iter()
        .enumerate()
        .map(|(index, kind)| {
            JournalEntry::record(
                format!("00000000-0000-0000-0000-{:012}", index + 200)
                    .parse::<EntryId>()
                    .expect("synthetic id"),
                "2026-10-10".parse().expect("date"),
                Note::new("Synthetic account filter").expect("note"),
                kind,
            )
            .expect("entry")
        })
        .collect();
    assert_eq!(
        history_entries(&entries, 0, 0, None, None, Some(bank)),
        entries[..3]
    );
    assert_eq!(
        history_entries(&entries, 0, 0, None, None, Some(card)),
        entries[..1]
    );
    assert_eq!(
        history_entries(&entries, 3, 0, None, None, Some(bank)),
        entries[..1]
    );
    assert_eq!(
        history_entries(&entries, 4, 0, None, None, Some(bank)),
        entries[1..3]
    );
    assert_eq!(
        history_entries(&entries, 1, 0, None, None, Some(other)),
        entries[3..4]
    );
    assert!(
        history_entries(
            &entries,
            0,
            0,
            None,
            None,
            Some(
                "00000000-0000-0000-0000-000000000104"
                    .parse::<AccountId>()
                    .expect("synthetic id")
            )
        )
        .is_empty()
    );
    assert_eq!(
        history_entries(&entries, 0, 0, None, None, None),
        entries[..4]
    );
}

#[test]
fn account_filter_composes_with_dates_year_type_and_precedes_pagination() {
    let selected = "00000000-0000-0000-0000-000000000001"
        .parse()
        .expect("account");
    let mut entries: Vec<_> = (1..=31)
        .map(|i| entry(i, &format!("2026-10-{i:02}"), false))
        .collect();
    entries.push(entry(32, "2026-10-15", true));
    entries.push(entry(33, "2025-10-15", false));
    let (from, through) = history_dates("2026-10-11", "2026-10-31").expect("range");
    let filtered = history_entries(&entries, 1, 2026, from, through, Some(selected));
    assert_eq!(filtered.len(), 21);
    assert_eq!(filtered[0].date().to_string(), "2026-10-31");
    assert_eq!(filtered[20].date().to_string(), "2026-10-11");
    assert_eq!(
        history_entries(&entries, 2, 2026, from, through, Some(selected)),
        entries[31..32]
    );
    assert_eq!(
        history_entries(&entries, 1, 2025, None, None, Some(selected)),
        entries[32..33]
    );
}

struct AccountGateway(Arc<AtomicUsize>);
impl UiGateway for AccountGateway {
    fn request(&self, command: Command) -> UiFuture<Response> {
        let requests = self.0.clone();
        Box::pin(async move {
            match command {
                Command::CreateAccount { opening, .. } => {
                    let _: Money = opening.parse()?;
                    requests.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(40)).await;
                    Ok(Response::Committed(CommitOutcome::Saved(
                        "00000000-0000-0000-0000-000000000001".parse()?,
                    )))
                }
                Command::Load => Ok(Response::Dashboard(dashboard(
                    LedgerState::default(),
                    "2026-10-04".parse()?,
                )?)),
                _ => Err(AppError::WorkerStopped),
            }
        })
    }
    fn save_csv(&self, _: CsvExport) -> UiFuture<Option<String>> {
        Box::pin(async { Ok(None) })
    }
    fn scan_receipt(
        &self,
        _: dioxus::html::FileData,
        _: Arc<AtomicBool>,
    ) -> UiFuture<(ReceiptImage, String)> {
        Box::pin(async { Err(AppError::WorkerStopped) })
    }
}

fn account_harness() -> Element {
    let mut store = crate::state::tests::use_test_state();
    use_context_provider(|| store);
    rsx! {
        button { id: "reopen", disabled: *store.busy.read(), onclick: move |_| store.account_form.set(true), "Reopen" }
        if *store.account_form.read() { AccountDialog {} }
        if !*store.busy.read() && store.view.read().is_some() { span { "finished-and-refreshed" } }
    }
}
#[allow(clippy::panic)]
fn field(changes: &Mutations, name: &str) -> ElementId {
    // Static DOM IDs live in Dioxus's template; listener mutations identify
    // the corresponding controls in this deliberately minimal harness.
    let (event, index) = match name {
        "reopen" => ("click", 0),
        "account-close" => ("click", 1),
        "account-dialog" => ("cancel", 0),
        "account-name" => ("input", 0),
        "opening" => ("input", 1),
        "account-create-form" => ("submit", 0),
        _ => panic!("unknown test control"),
    };
    changes
        .edits
        .iter()
        .filter_map(|m| match m {
            Mutation::NewEventListener { name, id } if *name == event => Some(*id),
            _ => None,
        })
        .nth(index)
        .expect("field")
}
fn event(dom: &mut VirtualDom, id: ElementId, name: &str, value: &str) -> Mutations {
    let data: Box<dyn Any> = if name == "click" {
        Box::new(dioxus::html::SerializedMouseData::default())
    } else if name == "cancel" {
        Box::new(dioxus::html::SerializedCancelData {})
    } else {
        Box::new(SerializedFormData::new(value.into(), vec![]))
    };
    dom.runtime().handle_event(
        name,
        Event::new(Rc::new(PlatformEventData::new(data)) as Rc<dyn Any>, true),
        id,
    );
    dom.render_immediate_to_vec()
}

#[tokio::test(flavor = "current_thread")]
async fn account_can_close_during_save_without_cancelling_or_duplicating_commit() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut dom = VirtualDom::new(account_harness);
    dom.insert_any_root_context(Box::new(Gateway(Arc::new(AccountGateway(calls.clone())))));
    let edits = dom.rebuild_to_vec();
    event(
        &mut dom,
        field(&edits, "account-name"),
        "input",
        "Synthetic bank",
    );
    event(&mut dom, field(&edits, "opening"), "input", "invalid");
    event(&mut dom, field(&edits, "account-create-form"), "submit", "");
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if dioxus_ssr::render(&dom).contains("class=\"form-error\"") {
                break;
            }
            dom.wait_for_work().await;
            dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
        }
    })
    .await
    .expect("validation returns control to the form");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    event(&mut dom, field(&edits, "opening"), "input", "01234.56");
    event(&mut dom, field(&edits, "account-create-form"), "submit", "");
    event(&mut dom, field(&edits, "account-create-form"), "submit", "");
    let html = dioxus_ssr::render(&dom);
    let close_tag = html
        .split("id=\"account-close\"")
        .nth(1)
        .expect("close")
        .split('>')
        .next()
        .expect("tag");
    assert!(
        !close_tag.contains("disabled"),
        "saving must not trap the user"
    );
    event(&mut dom, field(&edits, "account-close"), "click", "");
    assert!(!dioxus_ssr::render(&dom).contains("id=\"account-dialog\""));
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            dom.wait_for_work().await;
            dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
            if dioxus_ssr::render(&dom).contains("finished-and-refreshed") {
                break;
            }
        }
    })
    .await
    .expect("save survives closed dialog");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    event(&mut dom, field(&edits, "reopen"), "click", "");
    assert!(dioxus_ssr::render(&dom).contains("id=\"account-dialog\""));
}

#[tokio::test(flavor = "current_thread")]
async fn native_account_close_allows_reopening_without_a_save() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut dom = VirtualDom::new(account_harness);
    dom.insert_any_root_context(Box::new(Gateway(Arc::new(AccountGateway(calls.clone())))));
    let edits = dom.rebuild_to_vec();
    event(&mut dom, field(&edits, "account-dialog"), "cancel", "");
    assert!(!dioxus_ssr::render(&dom).contains("id=\"account-dialog\""));
    event(&mut dom, field(&edits, "reopen"), "click", "");
    assert!(dioxus_ssr::render(&dom).contains("id=\"account-dialog\""));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
