#![allow(clippy::expect_used)]
use super::*;
use crate::{Gateway, UiFuture, UiGateway};
use dioxus::dioxus_core::{AttributeValue, ElementId, Mutation, Mutations};
use dioxus::html::{PlatformEventData, SerializedFormData, set_event_converter};
use std::{
    any::Any,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::Duration,
};

const OLD: &str = "00000000-0000-0000-0000-000000000001";
const NEXT: &str = "00000000-0000-0000-0000-000000000002";
const BANK: &str = "00000000-0000-0000-0000-000000000003";
struct EditGateway(Arc<Mutex<LedgerState>>);
impl UiGateway for EditGateway {
    fn request(&self, command: Command) -> UiFuture<Response> {
        let state = self.0.clone();
        Box::pin(async move {
            match command {
                Command::Load => Ok(Response::Dashboard(dashboard(
                    state.lock().expect("state").clone(),
                    "2026-09-26".parse()?,
                )?)),
                Command::EditRecurring { expected, input } => {
                    let replacement =
                        input.validate("00000000-0000-0000-0000-000000000004".parse()?)?;
                    let mut state = state.lock().expect("state");
                    let (stopped, replacement) =
                        revised_recurring(&state, &expected, &replacement)?;
                    let slot = state
                        .recurring
                        .iter_mut()
                        .find(|s| s.id() == expected.id())
                        .expect("plan");
                    *slot = stopped;
                    state.recurring.push(replacement);
                    Ok(Response::PromptCommitted)
                }
                _ => Err(AppError::Input("Unexpected test command".into())),
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
fn fixture() -> LedgerState {
    let bank = BANK.parse().expect("id");
    let make = |id: &str, start: &str| {
        RecurringExpense::new(
            id.parse().expect("id"),
            AccountName::new("Synthetic rent").expect("name"),
            PositiveMoney::new("12200".parse().expect("money")).expect("amount"),
            Category::OtherExpense,
            Some(bank),
            MonthlyDue::new(16, start.parse().expect("month")).expect("due"),
        )
        .expect("plan")
    };
    LedgerState {
        accounts: vec![Account::new(
            bank,
            AccountName::new("Test bank").expect("name"),
            AccountKind::Bank,
        )],
        recurring: vec![
            make(OLD, "2026-09")
                .stop_from("2026-10".parse().expect("month"))
                .expect("stop"),
            make(NEXT, "2026-10"),
        ],
        ..Default::default()
    }
}
fn harness() -> Element {
    let mut store = crate::state::tests::use_test_state();
    use_context_provider(|| store);
    let initial = use_context::<Dashboard>();
    use_hook(move || store.view.set(Some(initial)));
    let confirmation = use_signal(|| None);
    use_context_provider(|| crate::confirmation::Confirmations(confirmation));
    let mut open = use_signal(|| true);
    let view = (store.view)().expect("view");
    let old = view
        .recurring
        .iter()
        .find(|s| s.id().to_string() == OLD)
        .expect("old")
        .clone();
    rsx! {
        if open() { EditRecurringDialog { schedule: old, effective: "2026-09".parse::<Month>().expect("month"), view, onclose: move |_| open.set(false) } }
        crate::confirmation::ConfirmationAlert {}
    }
}
fn ids(changes: &Mutations) -> HashMap<String, ElementId> {
    changes
        .edits
        .iter()
        .filter_map(|e| match e {
            Mutation::SetAttribute {
                name: "id",
                value: AttributeValue::Text(value),
                id,
                ..
            } => Some((value.clone(), *id)),
            _ => None,
        })
        .collect()
}
fn listener(changes: &Mutations, event: &str) -> ElementId {
    changes
        .edits
        .iter()
        .find_map(|e| match e {
            Mutation::NewEventListener { name, id } if *name == event => Some(*id),
            _ => None,
        })
        .expect("listener")
}
fn form_event(dom: &mut VirtualDom, id: ElementId, event: &str, value: &str) -> Mutations {
    let data = SerializedFormData::new(value.into(), vec![]);
    dom.runtime().handle_event(
        event,
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>,
            true,
        ),
        id,
    );
    dom.render_immediate_to_vec()
}
fn click(dom: &mut VirtualDom, id: ElementId) -> Mutations {
    let data = dioxus::html::SerializedMouseData::default();
    dom.runtime().handle_event(
        "click",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>,
            true,
        ),
        id,
    );
    dom.render_immediate_to_vec()
}

#[tokio::test(flavor = "current_thread")]
async fn change_month_select_future_plan_cancel_then_confirm_preserves_old_terms() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let state = Arc::new(Mutex::new(fixture()));
    let original = state.lock().expect("state").clone();
    let mut dom = VirtualDom::new(harness);
    dom.insert_any_root_context(Box::new(
        dashboard(original.clone(), "2026-09-26".parse().expect("date")).expect("view"),
    ));
    dom.insert_any_root_context(Box::new(Gateway(Arc::new(EditGateway(state.clone())))));
    let changes = dom.rebuild_to_vec();
    let fields = ids(&changes);
    let submit = listener(&changes, "submit");
    let initial = dioxus_ssr::render(&dom);
    assert!(
        initial.contains("max=\"9999-12\""),
        "month must not be capped at September: {initial}"
    );
    form_event(&mut dom, listener(&changes, "input"), "input", "2026-11");
    assert!(dioxus_ssr::render(&dom).contains("แผนที่เปิดมาไม่ได้ใช้ในเดือนนี้"));
    // Never guess the successor using its name; the user selects it explicitly.
    form_event(&mut dom, listener(&changes, "change"), "change", NEXT);
    form_event(&mut dom, fields["edit-rec-day"], "input", "28");
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("value=\"28\""));
    assert!(html.contains("value=\"2026-11\""));
    let prompt = form_event(&mut dom, submit, "submit", "");
    assert!(dioxus_ssr::render(&dom).contains("edit-confirmation"));
    assert_eq!(*state.lock().expect("state"), original);
    // Alert's second button is Cancel. No command may have reached storage.
    let clicks: Vec<_> = prompt
        .edits
        .iter()
        .filter_map(|e| match e {
            Mutation::NewEventListener { name, id } if name == "click" => Some(*id),
            _ => None,
        })
        .collect();
    click(&mut dom, clicks[1]);
    assert!(!dioxus_ssr::render(&dom).contains("id=\"edit-confirmation\""));
    assert_eq!(*state.lock().expect("state"), original);
    let prompt = form_event(&mut dom, submit, "submit", "");
    click(&mut dom, listener(&prompt, "click"));
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            dom.wait_for_work().await;
            dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
            if !dioxus_ssr::render(&dom).contains("id=\"rec-edit-dialog\"") {
                break;
            }
        }
    })
    .await
    .expect("commit refresh closes editor");
    let after = dashboard(
        state.lock().expect("state").clone(),
        "2026-09-26".parse().expect("date"),
    )
    .expect("view");
    for (period, day) in [
        ("2026-09", 16),
        ("2026-10", 16),
        ("2026-11", 28),
        ("2027-01", 28),
    ] {
        let monthly = recurring_month(&after, period.parse().expect("month")).expect("month");
        assert_eq!(monthly.items.len(), 1, "no duplicate bills in {period}");
        assert_eq!(monthly.items[0].schedule.due().day(), day, "{period}");
        assert_eq!(monthly.planned.to_string(), "12200.00");
    }
}
