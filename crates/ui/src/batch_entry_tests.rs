#![allow(clippy::expect_used, clippy::panic)]
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

const ACCOUNT: &str = "00000000-0000-0000-0000-000000000001";
#[derive(Default)]
struct Probe {
    next: u64,
    saved: Vec<PreparedEntry>,
    commits: usize,
    reject_next: bool,
}
struct TestGateway(Arc<Mutex<Probe>>);
fn fixture() -> Dashboard {
    dashboard(
        LedgerState {
            accounts: vec![Account::new(
                ACCOUNT.parse().expect("id"),
                AccountName::new("Synthetic cash").expect("name"),
                AccountKind::Cash,
            )],
            ..Default::default()
        },
        "2026-09-28".parse().expect("date"),
    )
    .expect("view")
}
fn prepare(input: EntryInput, probe: &mut Probe) -> Result<PreparedEntry, AppError> {
    probe.next += 1;
    let id = format!("00000000-0000-0000-0000-{:012}", probe.next);
    Ok(PreparedEntry {
        submission: id.parse()?,
        entry: JournalEntry::record(
            id.parse()?,
            input.date.parse()?,
            Note::new(&input.note)?,
            EntryKind::Expense {
                account: input.account.ok_or(AppError::WorkerStopped)?,
                amount: PositiveMoney::new(input.amount.parse()?)?,
                category: input.category.ok_or(AppError::WorkerStopped)?,
            },
        )?,
        recurring: None,
    })
}
impl UiGateway for TestGateway {
    fn request(&self, command: Command) -> UiFuture<Response> {
        let probe = self.0.clone();
        Box::pin(async move {
            let mut p = probe.lock().expect("probe");
            match command {
                Command::Load => Ok(Response::Dashboard(fixture())),
                Command::Preview(input) => Ok(Response::Prepared(prepare(input, &mut p)?)),
                Command::PreviewBatch(inputs) => Ok(Response::PreparedBatch(
                    inputs
                        .into_iter()
                        .map(|i| prepare(i, &mut p))
                        .collect::<Result<Vec<_>, _>>()?,
                )),
                Command::CommitBatch(entries) => {
                    p.commits += 1;
                    if p.reject_next {
                        p.reject_next = false;
                        return Err(AppError::Input("Synthetic commit failure".into()));
                    }
                    let result = entries
                        .iter()
                        .map(|entry| CommitOutcome::Saved(entry.entry.id()))
                        .collect();
                    p.saved = entries;
                    Ok(Response::CommittedBatch(result))
                }
                _ => Err(AppError::Input(
                    "Unexpected command in synthetic review test".into(),
                )),
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
fn harness() -> Element {
    let mut store = crate::state::tests::use_test_state();
    use_context_provider(|| store);
    use_hook(move || {
        store.view.set(Some(fixture()));
        store.batch.set(Some((
            "Synthetic notebook 80 and pen 20".into(),
            [("Notebook", "80"), ("Pen", "20")]
                .into_iter()
                .map(|(note, amount)| ModelDraft {
                    input: EntryInput {
                        amount: amount.into(),
                        note: note.into(),
                        account: Some(ACCOUNT.parse().expect("id")),
                        category: Some(Category::OtherExpense),
                        ..EntryInput::empty("2026-09-28".parse().expect("date"))
                    },
                    guidance: String::new(),
                })
                .collect(),
        )));
    });
    rsx! {
        if let Some((source, drafts)) = store.batch.read().clone() { BatchReview { source, drafts, view: fixture() } }
        else { p { "Batch recorded" } }
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

async fn settle(dom: &mut VirtualDom) -> Mutations {
    let mut edits = Vec::new();
    for _ in 0..8 {
        let _ = tokio::time::timeout(Duration::from_millis(15), dom.wait_for_work()).await;
        edits.extend(dom.render_immediate_to_vec().edits);
    }
    Mutations { edits }
}
#[tokio::test(flavor = "current_thread")]
async fn checks_and_inline_edits_keep_every_card_until_one_atomic_confirmation() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let probe = Arc::new(Mutex::new(Probe::default()));
    let mut dom = VirtualDom::new(harness);
    dom.insert_any_root_context(Box::new(Gateway(Arc::new(TestGateway(probe.clone())))));
    let mut fields = ids(&dom.rebuild_to_vec());
    form_event(&mut dom, fields["batch-reviewed-0"], "change", "true");
    click(&mut dom, fields["batch-confirm"]);
    settle(&mut dom).await;
    assert_eq!(
        probe.lock().expect("probe").commits,
        0,
        "one check cannot save or end a multi-entry review"
    );
    assert!(dioxus_ssr::render(&dom).contains("Pen"));
    fields.extend(ids(&click(&mut dom, fields["batch-edit-1"])));
    form_event(
        &mut dom,
        fields["batch-note-1"],
        "input",
        "Blue pen for work",
    );
    form_event(&mut dom, fields["batch-amount-1"], "input", "25.00");
    click(&mut dom, fields["batch-save-1"]);
    fields.extend(ids(&settle(&mut dom).await));
    let html = dioxus_ssr::render(&dom);
    assert!(
        html.contains("Notebook") && html.contains("Blue pen for work"),
        "{html}"
    );
    assert_eq!(
        probe.lock().expect("probe").commits,
        0,
        "saving a card must not record money"
    );
    click(&mut dom, fields["batch-confirm"]);
    click(&mut dom, fields["batch-confirm"]); // A double click must not queue a second write.
    settle(&mut dom).await;
    let p = probe.lock().expect("probe");
    assert_eq!(p.commits, 1);
    assert_eq!(p.saved.len(), 2);
    assert_eq!(p.saved[1].entry.note().as_str(), "Blue pen for work");
    assert!(dioxus_ssr::render(&dom).contains("Batch recorded"));
}

#[tokio::test(flavor = "current_thread")]
async fn failed_final_save_keeps_all_cards_and_retries_the_same_submission_ids() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let probe = Arc::new(Mutex::new(Probe {
        reject_next: true,
        ..Default::default()
    }));
    let mut dom = VirtualDom::new(harness);
    dom.insert_any_root_context(Box::new(Gateway(Arc::new(TestGateway(probe.clone())))));
    let fields = ids(&dom.rebuild_to_vec());
    for index in 0..2 {
        form_event(
            &mut dom,
            fields[&format!("batch-reviewed-{index}")],
            "change",
            "true",
        );
    }
    click(&mut dom, fields["batch-confirm"]);
    settle(&mut dom).await;
    assert!(dioxus_ssr::render(&dom).contains("Notebook"));
    assert!(probe.lock().expect("probe").saved.is_empty());
    let generated = probe.lock().expect("probe").next;
    click(&mut dom, fields["batch-confirm"]);
    settle(&mut dom).await;
    let p = probe.lock().expect("probe");
    assert_eq!(
        p.next, generated,
        "retry must reuse prepared submission IDs"
    );
    assert_eq!(p.saved.len(), 2);
}
