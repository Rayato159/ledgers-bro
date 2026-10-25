use crate::{components::*, state::*};
use dioxus::prelude::*;
use ledger_application::*;
use ledger_domain::*;

fn is_outstanding(item: &ReceivableProgress) -> bool {
    item.status != ReceivableStatus::Cancelled && item.outstanding > Money::ZERO
}

struct DebtorGroup {
    name: String,
    outstanding: Money,
    items: Vec<ReceivableProgress>,
}

fn debtor_groups(items: &[ReceivableProgress]) -> Result<Vec<DebtorGroup>, DomainError> {
    let mut groups = std::collections::BTreeMap::<String, DebtorGroup>::new();
    for item in items.iter().filter(|item| is_outstanding(item)) {
        let name = item
            .loan
            .debtor()
            .as_str()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let group = groups
            .entry(name.to_lowercase())
            .or_insert_with(|| DebtorGroup {
                name,
                outstanding: Money::ZERO,
                items: Vec::new(),
            });
        group.outstanding = group.outstanding.checked_add(item.outstanding)?;
        group.items.push(item.clone());
    }
    let mut groups: Vec<_> = groups.into_values().collect();
    groups.sort_by(|a, b| {
        b.outstanding
            .cmp(&a.outstanding)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(groups)
}

#[component]
fn DebtorTotals(items: Vec<ReceivableProgress>) -> Element {
    let mut store = use_context::<UiState>();
    let groups = match debtor_groups(&items) {
        Ok(groups) => groups,
        Err(error) => return rsx! { p { class: "form-error", role: "alert", "{error}" } },
    };
    let total = groups
        .iter()
        .try_fold(Money::ZERO, |sum, group| sum.checked_add(group.outstanding));
    let total = match total {
        Ok(total) => total,
        Err(error) => return rsx! { p { class: "form-error", role: "alert", "{error}" } },
    };
    rsx! {
        section { class: "card debtor-totals",
            div { class: "section-heading",
                h2 { {crate::i18n::tr("หนี้ค้างรวมตามคน")} }
                strong { id: "debtor-total-outstanding", "{crate::i18n::currency_prefix()}{money_label(total)}" }
            }
            p { class: "field-hint", {crate::i18n::tr("รวมยอดที่ยังไม่จ่ายของชื่อเดียวกัน ตามตัวกรองที่เลือก ไม่รวมรายการชำระครบหรือยกเลิก")} }
            if groups.is_empty() { p { class: "muted", {crate::i18n::tr("ไม่มีหนี้ค้างตามตัวกรองที่เลือก")} } }
            for group in groups {
                article { class: "debtor-group", key: "{group.name.to_lowercase()}",
                    div { class: "section-heading",
                        h3 { "{group.name}" }
                        strong { class: "debtor-group-total", "{crate::i18n::currency_prefix()}{money_label(group.outstanding)}" }
                    }
                    details {
                        summary { {crate::i18n::text("ดูหนี้ค้าง {0} รายการ", &[group.items.len().to_string()])} }
                        for item in group.items {
                            div { class: "debtor-group-item", key: "{item.loan.id()}",
                                div { strong { "{item.loan.description().as_str()}" } p { class: "field-hint", "{item.loan.opened()}" } }
                                strong { "{crate::i18n::currency_prefix()}{money_label(item.outstanding)}" }
                                button { class: "soft-button", disabled: *store.busy.read(), onclick: move |_| { store.receivable_review.set(None); store.repayment_selection.set(Some(item.loan.id())); store.repayment_form.set(true); }, {crate::i18n::tr("บันทึกลูกหนี้ชำระหนี้")} }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn filter_receivables(
    items: &mut Vec<ReceivableProgress>,
    name: &str,
    from: &str,
    through: &str,
) -> Result<(), &'static str> {
    let parse = |value: &str| {
        if value.is_empty() {
            Ok(None)
        } else {
            value
                .parse::<EntryDate>()
                .map(Some)
                .map_err(|_| "กรุณาระบุวันที่ให้ครบถ้วน")
        }
    };
    let dates = parse(from)
        .and_then(|from| parse(through).map(|through| (from, through)))
        .and_then(|(from, through)| {
            if from.zip(through).is_some_and(|(a, b)| a > b) {
                Err("วันที่เริ่มต้องไม่เกินวันที่สิ้นสุด")
            } else {
                Ok((from, through))
            }
        });
    let (from, through) = match dates {
        Ok(dates) => dates,
        Err(error) => {
            items.clear();
            return Err(error);
        }
    };
    let name = name.trim().to_lowercase();
    items.retain(|item| {
        item.loan.debtor().as_str().to_lowercase().contains(&name)
            && from.is_none_or(|date| item.loan.opened() >= date)
            && through.is_none_or(|date| item.loan.opened() <= date)
    });
    items.sort_by_key(|item| std::cmp::Reverse(item.loan.opened()));
    Ok(())
}

#[cfg(test)]
mod filter_tests {
    #![allow(clippy::expect_used)]
    use super::*;
    fn item(name: &str, date: &str) -> ReceivableProgress {
        let opened: EntryDate = date.parse().expect("date");
        ReceivableProgress {
            loan: Receivable::new(
                "00000000-0000-0000-0000-000000000001".parse().expect("id"),
                AccountName::new(name).expect("name"),
                Note::new("Synthetic share").expect("note"),
                PositiveMoney::new("10".parse().expect("amount")).expect("positive"),
                opened,
                CollectionTerms::new(Month::of(opened).expect("month"), None, None).expect("terms"),
            )
            .expect("loan"),
            paid: Money::ZERO,
            outstanding: "10".parse().expect("money"),
            overdue: Money::ZERO,
            paid_installments: None,
            remaining_installments: None,
            next_due: None,
            next_amount: None,
            status: ReceivableStatus::Unscheduled,
            payments: vec![],
        }
    }
    #[test]
    fn totals_group_only_unpaid_balances_and_keep_settled_and_cancelled_in_history() {
        let mut partial = item("Synthetic Alex", "2026-10-01");
        partial.paid = "3.25".parse().expect("paid");
        partial.outstanding = "6.75".parse().expect("remaining");
        let second = item("synthetic  ALEX", "2026-10-02");
        let mut settled = item("Synthetic Alex", "2026-10-03");
        settled.paid = "10".parse().expect("paid");
        settled.outstanding = Money::ZERO;
        settled.status = ReceivableStatus::Paid;
        let mut cancelled = item("Synthetic Cancelled", "2026-10-04");
        cancelled.outstanding = Money::ZERO;
        cancelled.status = ReceivableStatus::Cancelled;
        let items = vec![partial, second, settled.clone(), cancelled];
        let groups = debtor_groups(&items).expect("totals");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].outstanding.to_string(), "16.75");
        assert_eq!(groups[0].items.len(), 2);
        assert_eq!(items.iter().filter(|p| !is_outstanding(p)).count(), 2);
        let mut filtered = items.clone();
        filter_receivables(&mut filtered, "alex", "2026-10-01", "2026-10-01").expect("filter");
        assert_eq!(
            debtor_groups(&filtered).expect("filtered totals")[0]
                .outstanding
                .to_string(),
            "6.75"
        );
        // A repayment reversal makes the debt active again; no archive flag can
        // strand it in history after its live balance changes.
        settled.outstanding = "10".parse().expect("reopened");
        settled.paid = Money::ZERO;
        settled.status = ReceivableStatus::Unscheduled;
        assert!(is_outstanding(&settled));
        assert_eq!(
            debtor_groups(&[settled]).expect("reopened totals")[0]
                .outstanding
                .to_string(),
            "10.00"
        );
    }

    #[test]
    fn name_and_opened_range_combine_inclusively_and_clear_without_mutating_loans() {
        let all = vec![
            item("Synthetic Alex", "2026-09-30"),
            item("Synthetic ALEX", "2026-10-01"),
            item("Synthetic Sam", "2026-10-01"),
            item("ทดสอบ", "2026-10-02"),
        ];
        let mut items = all.clone();
        filter_receivables(&mut items, " alex ", "2026-09-30", "2026-10-01").expect("filter");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].loan.opened().to_string(), "2026-10-01");
        filter_receivables(&mut items, "Alex", "2026-10-01", "2026-10-01").expect("exact date");
        assert_eq!(items.len(), 1);
        let mut items = all.clone();
        filter_receivables(&mut items, "ทด", "", "").expect("Thai name");
        assert_eq!(items.len(), 1);
        let mut items = all.clone();
        assert!(filter_receivables(&mut items, "", "2026-10-02", "2026-09-30").is_err());
        assert!(items.is_empty());
        let mut items = all.clone();
        filter_receivables(&mut items, "", "", "").expect("clear");
        assert_eq!(items.len(), all.len());
    }
}

#[component]
pub(crate) fn ExpenseClaimDialog(source: JournalEntry, onclose: EventHandler<()>) -> Element {
    let mut store = use_context::<UiState>();
    let mut debtor = use_signal(String::new);
    let mut mode = use_signal(|| "equal".to_owned());
    let mut people = use_signal(|| "2".to_owned());
    let mut custom = use_signal(String::new);
    let mut submitted = use_signal(|| None::<ReceivableId>);
    use_effect(move || {
        if let Some(id) = submitted()
            && store
                .view
                .read()
                .as_ref()
                .is_some_and(|v| v.receivables.iter().any(|r| r.id() == id))
        {
            onclose.call(());
        }
    });
    let Some(view) = store.view.read().clone() else {
        return rsx! {};
    };
    let EntryKind::Expense { amount, .. } = source.kind() else {
        return rsx! {};
    };
    let total = amount.money();
    let remaining = expense_personal_amount(&view, &source).unwrap_or(Money::ZERO);
    let share = if mode() == "equal" {
        ExpenseShare::Equal(people().parse().unwrap_or(0))
    } else {
        ExpenseShare::Amount(custom())
    };
    let calculated = expense_share_amount(total, &share).ok().map(|v| v.money());
    let review = store.receivable_review.read().clone();
    let source_id = source.id();
    rsx! {
        dialog { id: "expense-claim-dialog", class: "account-dialog recurring-dialog", "aria-labelledby": "expense-claim-title",
            onmounted: move |_| { let _ = document::eval("document.getElementById('expense-claim-dialog').showModal(); document.getElementById('claim-debtor')?.focus()"); },
            oncancel: move |event| { event.prevent_default(); if !*store.busy.peek() { store.receivable_review.set(None); onclose.call(()); } },
            div { class: "section-heading",
                h2 { id: "expense-claim-title", {crate::i18n::tr("แบ่งรายจ่ายเป็นลูกหนี้")} }
                button { class: "icon-button", disabled: *store.busy.read(), "aria-label": crate::i18n::tr("ปิดหน้าต่าง"), onclick: move |_| { store.receivable_review.set(None); onclose.call(()); }, Icon { name: "close", size: 20 } }
            }
            p { class: "entry-note-body", "{source.date()} · {source.note().as_str()}" }
            p { {crate::i18n::text("ยอดเดิม {0} · ยังแบ่งได้ {1}", &[money_label(total), money_label(remaining)])} }
            if let Some((true, error)) = store.notice.read().clone() { p { class: "form-error", role: "alert", "{crate::i18n::tr(&error)}" } }
            if let Some(ReceivableReview::New(prepared)) = review {
                p { class: "batch-question", {crate::i18n::text("{0} ค้างเรา {1}", &[prepared.loan.debtor().as_str().to_owned(), money_label(prepared.loan.total().money())])} }
                p { {crate::i18n::text("ส่วนของเราหลังแบ่ง: {0}", &[money_label(remaining.checked_sub(prepared.loan.total().money()).unwrap_or(Money::ZERO))])} }
                p { class: "field-hint", {crate::i18n::tr("รายการนี้ไม่หักเงินในบัญชีซ้ำ เปลี่ยนส่วนที่คนอื่นค้างจากรายจ่ายเป็นลูกหนี้")} }
                div { class: "dialog-actions",
                    button { id: "claim-confirm", class: "primary", disabled: *store.busy.read(), onclick: move |_| { submitted.set(Some(prepared.loan.id())); store.send(Command::CreateReceivable(*prepared.clone())); }, {crate::i18n::tr("ยืนยันเพิ่มลูกหนี้")} }
                    button { class: "soft-button", disabled: *store.busy.read(), onclick: move |_| { store.receivable_review.set(None); store.notice.set(None); }, {crate::i18n::tr("กลับไปแก้")} }
                }
            } else {
                form { onsubmit: move |event| { event.prevent_default(); store.send(Command::PreviewExpenseClaim { source: source_id, debtor: debtor(), share: share.clone() }); },
                    fieldset { class: "recurring-fields", disabled: *store.busy.read(),
                        div { label { r#for: "claim-debtor", {crate::i18n::tr("ชื่อลูกหนี้")} } input { id: "claim-debtor", required: true, maxlength: 60, value: debtor(), oninput: move |event| debtor.set(event.value()) } }
                        div { label { r#for: "claim-mode", {crate::i18n::tr("วิธีแบ่งยอด")} }
                            select { id: "claim-mode", value: mode(), onchange: move |event| mode.set(event.value()),
                                option { value: "equal", selected: mode() == "equal", {crate::i18n::tr("หารเท่ากัน")} }
                                option { value: "amount", selected: mode() == "amount", {crate::i18n::tr("ระบุยอดเอง")} }
                            }
                        }
                        if mode() == "equal" {
                            div { label { r#for: "claim-people", {crate::i18n::tr("จำนวนคนรวมตัวเอง")} } input { id: "claim-people", r#type: "number", min: "2", max: "1000", required: true, value: people(), oninput: move |event| people.set(event.value()) } }
                        } else {
                            div { label { r#for: "claim-amount", {crate::i18n::tr("ยอดที่คนนี้ค้างเรา")} } input { id: "claim-amount", inputmode: "decimal", required: true, value: custom(), oninput: move |event| custom.set(event.value()) } }
                        }
                    }
                    if let Some(amount) = calculated { p { class: "batch-question", {crate::i18n::text("ลูกหนี้คนนี้: {0}", &[money_label(amount)])} } }
                    p { class: "field-hint", {crate::i18n::tr("เพิ่มลูกหนี้ทีละคนจากรายการเดิม หารจากยอดเต็ม เศษสตางค์อยู่ในส่วนของเรา")} }
                    p { class: "field-hint", {crate::i18n::tr("รายการนี้ไม่หักเงินในบัญชีซ้ำ เปลี่ยนส่วนที่คนอื่นค้างจากรายจ่ายเป็นลูกหนี้")} }
                    button { id: "claim-preview", r#type: "submit", class: "primary", disabled: *store.busy.read() || calculated.is_none_or(|v| v > remaining), {crate::i18n::tr("ตรวจข้อมูลลูกหนี้")} }
                }
            }
        }
    }
}

fn new_form(today: EntryDate) -> ReceivableInput {
    ReceivableInput {
        debtor: String::new(),
        description: String::new(),
        total: String::new(),
        opened: today.to_string(),
        start: format!("{:04}-{:02}", today.month_key().0, today.month_key().1),
        day: None,
        installments: None,
        source: None,
    }
}

fn edit_form(loan: &Receivable, opening: &JournalEntry) -> ReceivableInput {
    ReceivableInput {
        debtor: loan.debtor().as_str().into(),
        description: loan.description().as_str().into(),
        total: loan.total().money().to_string(),
        opened: loan.opened().to_string(),
        start: loan.start().to_string(),
        day: loan.day().map(|v| v.to_string()),
        installments: loan.installments().map(|v| v.to_string()),
        source: match opening.kind() {
            EntryKind::Lending { account, .. } => Some(*account),
            _ => None,
        },
    }
}

#[component]
pub(crate) fn ReceivablesPage(view: Dashboard) -> Element {
    let tab = use_signal(|| 0usize);
    let mut store = use_context::<UiState>();
    let mut adding = use_signal(|| false);
    let mut form = use_signal(|| new_form(view.today));
    let mut is_new_loan = use_signal(|| false);
    let mut known_count = use_signal(|| view.receivables.len());
    let mut filter_name = use_signal(String::new);
    let mut filter_from = use_signal(String::new);
    let mut filter_through = use_signal(String::new);
    use_effect(move || {
        if let Some(view) = store.view.read().as_ref()
            && view.receivables.len() != *known_count.peek()
        {
            known_count.set(view.receivables.len());
            adding.set(false);
            form.set(new_form(view.today));
            is_new_loan.set(false);
        }
    });
    let mut summary = match receivable_summary(&view) {
        Ok(value) => value,
        Err(e) => return rsx! { p { role: "alert", "{e}" } },
    };
    let filters = filter_receivables(
        &mut summary.items,
        &filter_name(),
        &filter_from(),
        &filter_through(),
    );
    let history = tab() == 2;
    if tab() != 1 {
        summary.items.retain(|item| is_outstanding(item) != history);
    }
    let list_tab = if history { 2 } else { 0 };
    let review = store.receivable_review.read().clone();
    let editing = store.receivable_edit.read().is_some();
    let from_expense = store
        .receivable_edit
        .read()
        .as_ref()
        .is_some_and(|(_, opening)| matches!(opening.kind(), EntryKind::ExpenseClaim { .. }));
    rsx! {
        section { class: "page-heading",
            div { h1 { {crate::i18n::text("ลูกหนี้", &[])} } p { class: "muted", {crate::i18n::text("ใครค้างเราเท่าไหร่ รับคืนแล้วกี่งวด และนัดเก็บเงินวันไหน", &[])} } }
            button { class: "primary", "aria-haspopup": "dialog", disabled: *store.busy.read(), onclick: move |_| { form.set(new_form(view.today)); is_new_loan.set(false); store.receivable_edit.set(None); adding.set(true); store.notice.set(None); store.receivable_review.set(None); }, {crate::i18n::text("เพิ่มลูกหนี้", &[])} }
        }
        PageTabs { id: "receivables", tabs: vec![("list", "หนี้ค้าง"), ("user", "รวมตามคน"), ("notebook", "ประวัติหนี้"), ("up", "ภาพรวมการรับคืน")], selected: tab }
        if tab() != 3 {
            div { class: "history-filters receivable-filters",
                div { label { r#for: "receivable-filter-name", {crate::i18n::tr("ค้นหาชื่อลูกหนี้")} } input { id: "receivable-filter-name", r#type: "search", value: filter_name(), oninput: move |e| filter_name.set(e.value()) } }
                div { label { r#for: "receivable-filter-from", {crate::i18n::tr("ตั้งหนี้ตั้งแต่วันที่")} } input { id: "receivable-filter-from", r#type: "date", value: filter_from(), onchange: move |e| filter_from.set(e.value()) } }
                div { label { r#for: "receivable-filter-through", {crate::i18n::tr("ถึงวันที่")} } input { id: "receivable-filter-through", r#type: "date", value: filter_through(), onchange: move |e| filter_through.set(e.value()) } }
                button { id: "receivable-filter-clear", class: "soft-button", disabled: filter_name().is_empty() && filter_from().is_empty() && filter_through().is_empty(), onclick: move |_| { filter_name.set(String::new()); filter_from.set(String::new()); filter_through.set(String::new()); }, {crate::i18n::tr("ล้างตัวกรอง")} }
            }
            if let Err(error) = filters { p { class: "form-error", role: "alert", "{crate::i18n::tr(error)}" } }
            if tab() != 1 { p { class: "history-result", role: "status", {crate::i18n::text("พบ {0} รายการ", &[summary.items.len().to_string()])} } }
        }
        PagePanel { lazy: true, id: "receivables", index: 1, selected: tab(), DebtorTotals { items: summary.items.clone() } }
        PagePanel { lazy: true, id: "receivables", index: 3, selected: tab(), ReceivablesChart { view: view.clone() } }
        if adding() || editing {
            dialog { id: "receivable-create-dialog", class: "account-dialog recurring-dialog receivable-dialog", "aria-labelledby": "receivable-create-title",
                onmounted: move |_| { let _ = document::eval("document.getElementById('receivable-create-dialog').showModal(); document.getElementById('debtor-name')?.focus()"); },
                oncancel: move |e| { e.prevent_default(); if !*store.busy.read() { adding.set(false); store.receivable_edit.set(None); store.receivable_review.set(None); store.notice.set(None); } },
                div { class: "section-heading",
                    h2 { id: "receivable-create-title", tabindex: "-1", {crate::i18n::tr(if editing { "แก้ไขลูกหนี้" } else { "เพิ่มรายการหนี้ที่คนอื่นติดเรา" })} }
                    button { class: "icon-button", disabled: *store.busy.read(), "aria-label": crate::i18n::tr("ปิดหน้าต่าง"), onclick: move |_| { adding.set(false); store.receivable_edit.set(None); store.receivable_review.set(None); store.notice.set(None); }, Icon { name: "close", size: 20 } }
                }
                if let Some((true, message)) = store.notice.read().clone() {
                    p { class: "form-error", role: "alert", "{crate::i18n::tr(&message)}" }
                    if editing {
                        button { class: "text-button", r#type: "button", disabled: *store.busy.read(), onclick: move |_| {
                            let selected = store.receivable_edit.peek().as_ref().map(|(loan, _)| loan.id());
                            let latest = store.view.peek().clone();
                            if let (Some(id), Some(latest)) = (selected, latest)
                                && let Some(loan) = latest.receivables.iter().find(|loan| loan.id() == id)
                                && let Some(opening) = latest.entries.iter().find(|e| matches!(e.kind(), EntryKind::Lending { receivable, .. } | EntryKind::ReceivableOpening { receivable, .. } | EntryKind::ExpenseClaim { receivable, .. } if *receivable == id)) {
                                let input = edit_form(loan, opening);
                                is_new_loan.set(input.source.is_some()); form.set(input);
                                store.receivable_edit.set(Some((loan.clone(), opening.clone())));
                                store.receivable_review.set(None); store.notice.set(None);
                            }
                        }, {crate::i18n::tr("โหลดรายการนี้ใหม่")} }
                    }
                }
                if let Some(ReceivableReview::Edit(prepared)) = review.clone() {
                    ReceivableEditReview { prepared: *prepared, view: view.clone() }
                } else if let Some(ReceivableReview::New(prepared)) = review {
                    div { class: "receivable-review-body", onmounted: move |_| { let _ = document::eval("document.getElementById('receivable-create-title').focus(); document.getElementById('receivable-create-dialog').scrollTop = 0"); },
                    p { "{prepared.loan.debtor().as_str()} · {prepared.loan.description().as_str()}" }
                    p { {crate::i18n::text("เงินต้น ฿{0} · ตั้งหนี้ {1}", &[money_label(prepared.loan.total().money()).to_string(), format!("{}", prepared.loan.opened())])} }
                    if let Some(n) = prepared.loan.installments() { p { {crate::i18n::text("กำหนด {0} งวด · เริ่ม {1}", &[format!("{}", n), format!("{}", prepared.loan.start())])} } } else { p { {crate::i18n::text("ไม่กำหนดจำนวนงวด", &[])} } }
                    if let Some(day) = prepared.loan.day() { p { {crate::i18n::text("เก็บทุกวันที่ {0} ของเดือน", &[format!("{}", day)])} } } else { p { {crate::i18n::text("ไม่กำหนดวันเก็บ", &[])} } }
                    if let EntryKind::Lending { account, .. } = prepared.opening.entry.kind() { p { class: "batch-question", {crate::i18n::text("จะลดเงินในบัญชี {0} และเพิ่มสินทรัพย์ลูกหนี้เท่ากัน", &[account_label(&view, *account).to_string()])} } }
                    else { p { class: "batch-question", {crate::i18n::text("เพิ่มยอดลูกหนี้ที่ค้างอยู่เป็นสินทรัพย์ยกมา เงินในบัญชีปัจจุบันไม่เปลี่ยน ใช้ยอดที่ยังค้าง ณ วันที่ตั้งหนี้", &[])} } }
                    div { class: "receivable-dialog-actions",
                        button { class: "primary", disabled: *store.busy.read(), onclick: move |_| store.send(Command::CreateReceivable((*prepared).clone())), {crate::i18n::text("ยืนยันเพิ่มลูกหนี้", &[])} }
                        button { class: "soft-button", disabled: *store.busy.read(), onclick: move |_| { store.notice.set(None); store.receivable_review.set(None); }, {crate::i18n::text("กลับไปแก้", &[])} }
                    }
                    }
                } else {
                    if editing { p { class: "field-hint", {crate::i18n::tr("แก้ข้อมูลที่กรอกผิดโดยคงประวัติรับชำระไว้ เงินต้นต้องรวมส่วนที่รับคืนแล้ว และต้องไม่น้อยกว่ายอดรับคืน")} } }
                    form { onsubmit: move |e| {
                        e.prevent_default();
                        if is_new_loan() && form.read().source.is_none() { store.notice.set(Some((true, "กรุณาเลือกบัญชีที่จ่ายเงินให้ยืม".into()))); }
                        else if let Some((expected, original)) = store.receivable_edit.peek().clone() {
                            store.send(Command::PreviewReceivableEdit { expected, original, input: form() });
                        } else { store.send(Command::PreviewReceivable(form())); }
                    },
                        fieldset { class: "recurring-fields", disabled: *store.busy.read(),
                            div { label { r#for: "debtor-name", {crate::i18n::text("ชื่อลูกหนี้", &[])} } input { id: "debtor-name", required: true, maxlength: 60, value: form.read().debtor.clone(), onmounted: move |_| { let _ = document::eval("document.getElementById('debtor-name')?.focus()"); }, oninput: move |e| form.write().debtor = e.value() } }
                            div { label { r#for: "debt-description", {crate::i18n::text("หนี้อะไร", &[])} } input { id: "debt-description", required: true, maxlength: 300, placeholder: crate::i18n::text("เช่น ยืมซื้อคอมพิวเตอร์", &[]), value: form.read().description.clone(), oninput: move |e| form.write().description = e.value() } }
                            div { label { r#for: "debt-total", {crate::i18n::text("เงินต้นทั้งหมดที่ยังค้าง ณ วันที่ตั้งหนี้ (บาท)", &[])} } input { id: "debt-total", required: true, inputmode: "decimal", value: form.read().total.clone(), oninput: move |e| form.write().total = e.value() } }
                            div { label { r#for: "debt-opened", {crate::i18n::text("วันที่ตั้งยอดหนี้", &[])} } input { id: "debt-opened", disabled: from_expense, r#type: "date", required: true, min: "1900-01-01", max: "{view.today}", value: form.read().opened.clone(), onchange: move |e| form.write().opened = e.value() } }
                            div { label { r#for: "debt-start", {crate::i18n::text("เดือนเริ่มเก็บ (ค.ศ.)", &[])} } input { id: "debt-start", r#type: "month", required: true, min: "1900-01", max: "9999-12", value: form.read().start.clone(), onchange: move |e| form.write().start = e.value() } }
                            div {
                                label { class: "flow-mode", input { r#type: "checkbox", checked: form.read().installments.is_some(), onchange: move |e| form.write().installments = e.checked().then(String::new) } {crate::i18n::text("กำหนดจำนวนงวด", &[])} }
                                if let Some(count) = form.read().installments.clone() { label { r#for: "debt-count", {crate::i18n::text("จำนวนงวดที่ต้องชำระ", &[])} } input { id: "debt-count", r#type: "number", min: "1", max: "1200", required: true, value: count, oninput: move |e| form.write().installments = Some(e.value()) } }
                                else { p { class: "field-hint", {crate::i18n::text("ไม่กำหนดจำนวนงวด รับคืนไปเรื่อย ๆ จนเงินต้นครบ", &[])} } }
                            }
                            div {
                                label { class: "flow-mode", input { r#type: "checkbox", checked: form.read().day.is_some(), onchange: move |e| form.write().day = e.checked().then(String::new) } {crate::i18n::text("กำหนดวันเก็บทุกเดือน", &[])} }
                                if let Some(day) = form.read().day.clone() { label { r#for: "debt-day", {crate::i18n::text("เก็บทุกวันที่ (1–31)", &[])} } input { id: "debt-day", r#type: "number", min: "1", max: "31", required: true, value: day, oninput: move |e| form.write().day = Some(e.value()) } }
                                else { p { class: "field-hint", {crate::i18n::text("ไม่กำหนดวันเก็บ จะไม่ตีความว่าเลยกำหนด", &[])} } }
                            }
                            div {
                                label { class: "flow-mode", input { r#type: "checkbox", checked: is_new_loan(), disabled: from_expense, onchange: move |e| { is_new_loan.set(e.checked()); form.write().source = None; } } {crate::i18n::text("เป็นการให้ยืมเงินใหม่ หักเงินจากบัญชี", &[])} }
                                if is_new_loan() {
                                    label { r#for: "debt-source", {crate::i18n::text("บัญชีที่จ่ายเงินให้ยืม", &[])} }
                                    select { id: "debt-source", required: true, value: form.read().source.map(|id| id.to_string()).unwrap_or_default(), onchange: move |e| form.write().source = e.value().parse().ok(),
                                        option { value: "", selected: form.read().source.is_none(), {crate::i18n::text("เลือกบัญชี", &[])} }
                                        for a in view.accounts.iter().filter(|a| a.account.accepts_cash_entries()) { option { value: "{a.account.id()}", selected: form.read().source == Some(a.account.id()), "{a.account.name().as_str()}" } }
                                    }
                                } else { p { class: "field-hint", {crate::i18n::text("หนี้ที่มีอยู่แล้ว: เพิ่มเฉพาะยอดลูกหนี้ เงินในบัญชีไม่เปลี่ยน", &[])} } }
                            }
                        }
                        p { class: "field-hint", {crate::i18n::text("เมื่อกำหนดงวด จะแบ่งเงินต้นเท่ากันและเก็บเศษสตางค์ในงวดสุดท้าย · รับชำระบางส่วนได้ โดยตัดงวดเก่าก่อน · วันที่ 29–31 ที่ไม่มีจะใช้วันสุดท้ายของเดือน", &[])} }
                        div { class: "receivable-dialog-actions",
                            button { class: "primary", r#type: "submit", disabled: *store.busy.read(), {crate::i18n::text("ตรวจข้อมูลลูกหนี้", &[])} }
                            button { class: "soft-button", r#type: "button", disabled: *store.busy.read(), onclick: move |_| { adding.set(false); store.receivable_edit.set(None); store.receivable_review.set(None); store.notice.set(None); }, {crate::i18n::tr("ยกเลิก")} }
                        }
                    }
                }
            }
        }
        PagePanel { lazy: true, id: "receivables", index: list_tab, selected: tab(), section { class: "card recurring-card",
            h2 { {crate::i18n::tr(if history { "ประวัติหนี้ที่ชำระครบหรือยกเลิก" } else { "รายการหนี้ที่ยังไม่จ่ายครบ" })} }
            if summary.items.is_empty() { p { class: "muted", {crate::i18n::tr(if history { "ไม่มีประวัติหนี้ตามตัวกรองที่เลือก" } else { "ไม่มีหนี้ค้างตามตัวกรองที่เลือก" })} } }
            for p in summary.items {
                article { class: "debtor-item", key: "{p.loan.id()}",
                    div { class: "section-heading", h3 { "{p.loan.debtor().as_str()}" } span { class: if p.status == ReceivableStatus::Overdue { "status-pill debt-overdue" } else { "status-pill" }, "{crate::i18n::tr(p.status.label())}" } }
                    p { "{p.loan.description().as_str()}" }
                    p { class: "field-hint", {crate::i18n::text("วันที่ตั้งยอดหนี้: {0}", &[p.loan.opened().to_string()])} }
                    div { class: "debtor-balance-grid",
                        div { small { {crate::i18n::text("ยังค้างเรา", &[])} } strong { "{crate::i18n::currency_prefix()}{money_label(p.outstanding)}" } }
                        div { small { {crate::i18n::text("รับคืนแล้ว", &[])} } strong { "{crate::i18n::currency_prefix()}{money_label(p.paid)}" } }
                    }
                    progress { class: "installment-progress", max: "{p.loan.total().money().minor()}", value: "{p.paid.minor()}", "aria-label": crate::i18n::text("ความคืบหน้าการรับคืนเงินต้น", &[]) }
                    if let (Some(paid), Some(remaining)) = (p.paid_installments, p.remaining_installments) { p { {crate::i18n::text("ครบแล้ว {0} งวด · เหลือ {1} งวด", &[format!("{}", paid), format!("{}", remaining)])} } }
                    else { p { {crate::i18n::text("ไม่กำหนดจำนวนงวด · รับชำระแล้ว {0} ครั้ง", &[format!("{}", p.payments.len())])} } }
                    if let Some(day) = p.loan.day() { p { {crate::i18n::text("เก็บทุกวันที่ {0} · เริ่ม {1}", &[format!("{}", day), format!("{}", p.loan.start())])} } } else { p { {crate::i18n::text("ไม่กำหนดวันเก็บ", &[])} } }
                    if let Some(due) = p.next_due { p { class: "next-collection", Icon { name: "calendar", size: 20 } {crate::i18n::text("นัดเก็บถัดไป / งวดค้างแรก: {0}", &[format!("{}", due)])} } }
                    if let Some(amount) = p.next_amount { p { {crate::i18n::text("ยอดที่ยังขาดของงวดถัดไป ฿{0}", &[money_label(amount).to_string()])} } }
                    if p.overdue > Money::ZERO { p { class: "form-error", {crate::i18n::text("เงินต้นเลยกำหนด ฿{0}", &[money_label(p.overdue).to_string()])} } }
                    div { class: "receivable-item-actions",
                    if p.status != ReceivableStatus::Cancelled {
                        button { class: "soft-button receivable-edit-button", r#type: "button", disabled: *store.busy.read(),
                            onclick: { let loan = p.loan.clone(); let opening = view.entries.iter().find(|e| matches!(e.kind(), EntryKind::Lending { receivable, .. } | EntryKind::ReceivableOpening { receivable, .. } | EntryKind::ExpenseClaim { receivable, .. } if *receivable == loan.id())).cloned(); move |_| {
                                if let Some(opening) = opening.clone() {
                                    let input = edit_form(&loan, &opening);
                                    is_new_loan.set(input.source.is_some()); form.set(input);
                                    adding.set(false); store.notice.set(None); store.receivable_review.set(None);
                                    store.receivable_edit.set(Some((loan.clone(), opening)));
                                }
                            } },
                            Icon { name: "edit", size: 18 } {crate::i18n::tr("แก้ไขลูกหนี้")}
                        }
                    }
                    if p.outstanding > Money::ZERO {
                        button { class: "primary", disabled: *store.busy.read(), onclick: { let id = p.loan.id(); move |_| { store.receivable_review.set(None); store.repayment_selection.set(Some(id)); store.repayment_form.set(true); } }, Icon { name: "down", size: 18 } {crate::i18n::text("บันทึกลูกหนี้ชำระหนี้", &[])} }
                    }
                    }
                    details { class: "flow-explanation", summary { {crate::i18n::text("ประวัติรับคืนเงินต้น ({0})", &[format!("{}", p.payments.len())])} }
                        for entry in p.payments { if let EntryKind::Repayment { account, amount, .. } = entry.kind() { p { "{entry.date()} · {account_label(&view, *account)} · {crate::i18n::currency_prefix()}{money_label(amount.money())}" } } }
                        button { class: "text-button", onclick: move |_| store.page.set(Page::Transactions), {crate::i18n::text("ดูรายการ / ยกเลิกรายการที่บันทึกผิด", &[])} }
                    }
                }
            }
        } }
    }
}

#[component]
fn ReceivableEditReview(prepared: PreparedReceivableEdit, view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let before = edit_form(prepared.expected(), prepared.original());
    let after = edit_form(prepared.loan(), prepared.opening());
    let paid = receivable_summary(&view)
        .ok()
        .and_then(|s| {
            s.items
                .into_iter()
                .find(|p| p.loan.id() == prepared.loan().id())
        })
        .map(|p| p.paid)
        .unwrap_or(Money::ZERO);
    let outstanding = prepared
        .loan()
        .total()
        .money()
        .checked_sub(paid)
        .unwrap_or(Money::ZERO);
    let source_label = |id: Option<AccountId>| {
        id.map(|id| account_label(&view, id))
            .unwrap_or_else(|| crate::i18n::tr("หนี้ที่มีอยู่แล้ว"))
    };
    let rows = vec![
        ("ชื่อลูกหนี้", before.debtor, after.debtor),
        ("หนี้อะไร", before.description, after.description),
        ("เงินต้นทั้งหมด", before.total, after.total),
        ("วันที่ตั้งยอดหนี้", before.opened, after.opened),
        ("เดือนเริ่มเก็บ (ค.ศ.)", before.start, after.start),
        (
            "เก็บทุกวันที่ (1–31)",
            before
                .day
                .unwrap_or_else(|| crate::i18n::tr("ไม่กำหนดวันเก็บ")),
            after.day.unwrap_or_else(|| crate::i18n::tr("ไม่กำหนดวันเก็บ")),
        ),
        (
            "จำนวนงวดที่ต้องชำระ",
            before
                .installments
                .unwrap_or_else(|| crate::i18n::tr("ไม่กำหนดจำนวนงวด")),
            after
                .installments
                .unwrap_or_else(|| crate::i18n::tr("ไม่กำหนดจำนวนงวด")),
        ),
        (
            "บัญชีที่จ่ายเงินให้ยืม",
            source_label(before.source),
            source_label(after.source),
        ),
    ];
    let balances: Vec<_> = view
        .accounts
        .iter()
        .filter_map(|a| {
            let amount = |entry: &JournalEntry| {
                entry
                    .postings()
                    .iter()
                    .find(|p| p.target() == PostingTarget::Account(a.account.id()))
                    .map(|p| p.amount())
                    .unwrap_or(Money::ZERO)
            };
            let delta = amount(prepared.opening())
                .checked_sub(amount(prepared.original()))
                .ok()?;
            if delta == Money::ZERO {
                return None;
            }
            Some((
                a.account.name().as_str().to_string(),
                a.balance,
                a.balance.checked_add(delta).ok()?,
            ))
        })
        .collect();
    rsx! {
        div { class: "receivable-edit-review", onmounted: move |_| { let _ = document::eval("document.getElementById('receivable-create-title').focus(); document.getElementById('receivable-create-dialog').scrollTop = 0"); },
            h3 { {crate::i18n::tr("ตรวจสอบการแก้ไขลูกหนี้")} }
            dl { class: "receivable-edit-changes",
                for (label, old, new) in rows {
                    div { dt { "{crate::i18n::tr(label)}" } dd { if old != new { span { class: "muted", "{old}" } span { " → " } } strong { "{new}" } } }
                }
            }
            div { class: "debtor-balance-grid",
                div { small { {crate::i18n::tr("รับคืนแล้ว")} } strong { "{crate::i18n::currency_prefix()}{money_label(paid)}" } }
                div { small { {crate::i18n::tr("ยังค้างเรา")} } strong { "{crate::i18n::currency_prefix()}{money_label(outstanding)}" } }
            }
            if matches!(prepared.original().kind(), EntryKind::ExpenseClaim { .. }) { p { class: "field-hint", {crate::i18n::tr("แก้ส่วนแบ่งจากรายจ่ายเดิม เงินในบัญชีและยอดหนี้บัตรไม่เปลี่ยน")} } }
            if balances.is_empty() { p { class: "field-hint", {crate::i18n::tr("ยอดเงินในบัญชีไม่เปลี่ยน")} } }
            else {
                h3 { {crate::i18n::tr("ยอดบัญชีหลังแก้ไข")} }
                for (name, old, new) in balances { p { "{name}: {crate::i18n::currency_prefix()}{money_label(old)} → {money_label(new)}" } }
            }
            p { class: "field-hint", {crate::i18n::tr("แก้ยอดตั้งหนี้เดิมและคำนวณยอดค้างใหม่ ประวัติรับเงินต้นและดอกเบี้ยยังอยู่ครบ")} }
            div { class: "receivable-dialog-actions",
                button { class: "primary", id: "confirm-receivable-edit", disabled: *store.busy.read(), onclick: move |_| store.send(Command::EditReceivable(Box::new(prepared.clone()))), {crate::i18n::tr("ยืนยันการแก้ไข")} }
                button { class: "soft-button", disabled: *store.busy.read(), onclick: move |_| { store.receivable_review.set(None); store.notice.set(None); }, {crate::i18n::tr("กลับไปแก้")} }
                button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| { store.receivable_edit.set(None); store.receivable_review.set(None); store.notice.set(None); }, {crate::i18n::tr("ยกเลิก")} }
            }
        }
    }
}

#[component]
pub(crate) fn RepaymentShortcut() -> Element {
    let mut store = use_context::<UiState>();
    rsx! { button { class: "text-button repayment-shortcut", r#type: "button", disabled: *store.busy.read(), onclick: move |_| { store.receivable_review.set(None); store.repayment_selection.set(None); store.repayment_form.set(true); }, Icon { name: "down", size: 18 } {crate::i18n::text("รับชำระหนี้จากลูกหนี้", &[])} } }
}

#[component]
pub(crate) fn RepaymentDialog(view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let mut selected = use_signal(|| *store.repayment_selection.peek());
    let mut account = use_signal(|| None::<AccountId>);
    let mut principal = use_signal(String::new);
    let mut interest = use_signal(|| "0".to_string());
    let mut date = use_signal(|| view.today.to_string());
    let summary = match receivable_summary(&view) {
        Ok(v) => v,
        Err(e) => return rsx! { p { role: "alert", "{e}" } },
    };
    let selected_progress = summary
        .items
        .iter()
        .find(|p| Some(p.loan.id()) == selected())
        .cloned();
    let review = store.receivable_review.read().clone();
    rsx! {
        dialog { id: "repayment-dialog", class: "account-dialog recurring-dialog", "aria-label": crate::i18n::text("บันทึกลูกหนี้ชำระหนี้", &[]),
            onmounted: move |_| { let _ = document::eval("document.getElementById('repayment-dialog').showModal()"); },
            oncancel: move |e| { e.prevent_default(); if !*store.busy.read() { store.repayment_form.set(false); store.receivable_review.set(None); } },
            h2 { {crate::i18n::text("บันทึกลูกหนี้ชำระหนี้", &[])} }
            if let Some((true, message)) = store.notice.read().clone() { p { class: "form-error", role: "alert", "{message}" } }
            if let Some(ReceivableReview::Payment(prepared)) = review {
                for p in &prepared { if let Some((label, amount, id)) = entry_label(&p.entry) { p { "{crate::i18n::tr(&label)} {crate::i18n::currency_prefix()}{money_label(amount)} · {account_label(&view, id)} · {p.entry.date()}" } } }
                if let Some(progress) = selected_progress { p { "{progress.loan.debtor().as_str()} · {progress.loan.description().as_str()}" } }
                p { class: "batch-question", {crate::i18n::text("เงินต้นจะลดยอดลูกหนี้และเพิ่มเงินในบัญชี ส่วนดอกเบี้ยจะเป็นรายรับแยก ยืนยันเมื่อได้รับเงินจริงแล้ว", &[])} }
                button { class: "primary", disabled: *store.busy.read(), onclick: move |_| store.send(Command::ReceiveRepayment(prepared.clone())), {crate::i18n::text("ยืนยันรับชำระและบันทึก", &[])} }
                button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.receivable_review.set(None), {crate::i18n::text("กลับไปแก้", &[])} }
            } else {
                form { onsubmit: move |e| { e.prevent_default(); if let Some(receivable) = selected() { store.send(Command::PreviewRepayment(RepaymentInput { receivable, account: account(), principal: principal(), interest: interest(), date: date() })); } else { store.notice.set(Some((true, "กรุณาเลือกลูกหนี้".into()))); } },
                    fieldset { class: "repayment-fields", disabled: *store.busy.read(),
                        label { r#for: "repayment-debtor", {crate::i18n::text("เลือกลูกหนี้ / เรื่องหนี้", &[])} }
                        select { id: "repayment-debtor", required: true, value: selected().map(|id| id.to_string()).unwrap_or_default(), onchange: move |e| selected.set(e.value().parse().ok()),
                            option { value: "", selected: selected().is_none(), {crate::i18n::text("เลือกลูกหนี้", &[])} }
                            for p in summary.items.iter().filter(|p| p.outstanding > Money::ZERO) { option { value: "{p.loan.id()}", selected: selected() == Some(p.loan.id()), {crate::i18n::text("{0} · {1} · ค้าง ฿{2}", &[p.loan.debtor().as_str().to_string(), p.loan.description().as_str().to_string(), money_label(p.outstanding).to_string()])} } }
                        }
                        if let Some(p) = selected_progress { p { class: "field-hint", {crate::i18n::text("ค้างเงินต้น ฿{0} · รับบางส่วนหรือปิดยอดทั้งหมดได้", &[money_label(p.outstanding).to_string()])} } }
                        label { r#for: "repayment-principal", {crate::i18n::text("เงินต้นที่ได้รับคืน (บาท)", &[])} } input { id: "repayment-principal", required: true, inputmode: "decimal", value: "{principal}", oninput: move |e| principal.set(e.value()) }
                        label { r#for: "repayment-interest", {crate::i18n::text("ดอกเบี้ยที่ได้รับ (ไม่มีใส่ 0)", &[])} } input { id: "repayment-interest", required: true, inputmode: "decimal", value: "{interest}", oninput: move |e| interest.set(e.value()) }
                        label { r#for: "repayment-account", {crate::i18n::text("บัญชีที่รับเงิน", &[])} }
                        select { id: "repayment-account", required: true, value: account().map(|id| id.to_string()).unwrap_or_default(), onchange: move |e| account.set(e.value().parse().ok()),
                            option { value: "", selected: account().is_none(), {crate::i18n::text("เลือกบัญชีรับเงิน", &[])} }
                            for a in view.accounts.iter().filter(|a| a.account.accepts_cash_entries()) { option { value: "{a.account.id()}", selected: account() == Some(a.account.id()), "{a.account.name().as_str()}" } }
                        }
                        label { r#for: "repayment-date", {crate::i18n::text("วันที่ได้รับเงิน", &[])} } input { id: "repayment-date", r#type: "date", required: true, min: "1900-01-01", max: "{view.today}", value: "{date}", onchange: move |e| date.set(e.value()) }
                    }
                    p { class: "field-hint", {crate::i18n::text("รับคืนเงินต้นไม่ใช่รายได้ใหม่ ระบบไม่บวกเงินต้นซ้ำในรายรับ / Flow rate และไม่สมมติว่าจะเก็บหนี้ได้ก่อนรับเงินจริง", &[])} }
                    button { class: "primary", r#type: "submit", disabled: *store.busy.read(), {crate::i18n::text("ตรวจรายการรับชำระ", &[])} }
                }
            }
            button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| { store.repayment_form.set(false); store.receivable_review.set(None); }, {crate::i18n::text("ปิด", &[])} }
        }
    }
}

#[component]
pub(crate) fn ReceivablesChart(view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let summary = match receivable_summary(&view) {
        Ok(v) => v,
        Err(e) => return rsx! { p { role: "alert", "{e}" } },
    };
    let counts: Vec<_> = [
        ReceivableStatus::Collecting,
        ReceivableStatus::Overdue,
        ReceivableStatus::Unscheduled,
        ReceivableStatus::Paid,
    ]
    .into_iter()
    .map(|s| (s, summary.items.iter().filter(|p| p.status == s).count()))
    .collect();
    let total = summary.total.minor();
    rsx! {
        section { class: "card recurring-card receivables-chart", "aria-label": crate::i18n::text("สถานะลูกหนี้โดยรวม", &[]),
            div { class: "section-heading", h2 { {crate::i18n::text("สถานะลูกหนี้โดยรวม", &[])} } button { class: "text-button", onclick: move |_| store.page.set(Page::Receivables), {crate::i18n::text("จัดการลูกหนี้ →", &[])} } }
            div { class: "debt-summary-visual",
                crate::debt_visuals::ProgressRing { done: summary.paid.minor(), total, label: crate::i18n::text("รับคืนแล้ว", &[]) }
                div { class: "debt-summary-bars",
                    crate::debt_visuals::AmountBar { label: crate::i18n::text("เงินต้นทั้งหมด", &[]), amount: summary.total, maximum: total, tone: "asset" }
                    crate::debt_visuals::AmountBar { label: crate::i18n::text("รับคืนเงินต้นแล้ว", &[]), amount: summary.paid, maximum: total, tone: "paid" }
                    crate::debt_visuals::AmountBar { label: crate::i18n::text("ยังค้างเรา", &[]), amount: summary.outstanding, maximum: total, tone: "plan" }
                    crate::debt_visuals::AmountBar { label: crate::i18n::text("ในยอดค้าง: เลยกำหนด", &[]), amount: summary.overdue, maximum: total, tone: "due" }
                }
            }
            if total == 0 { p { class: "muted", {crate::i18n::text("ยังไม่มียอดลูกหนี้ให้ติดตาม", &[])} } }
            div { class: "debt-status-grid", for (status, n) in counts {
                div { class: "debt-status-stat", "data-overdue": status == ReceivableStatus::Overdue,
                    div { strong { "{n}" } span { "{crate::i18n::tr(status.label())}" } }
                    div { class: "debt-bar-track", "aria-hidden": "true", span { style: "width:{crate::debt_visuals::percentage(n as i64, summary.items.iter().filter(|p| p.status != ReceivableStatus::Cancelled).count() as i64)}%;" } }
                }
            } }
            details { class: "flow-explanation", summary { {crate::i18n::text("วิธีนับยอดลูกหนี้", &[])} }
                p { class: "field-hint", {crate::i18n::text("นับแยกตามเรื่องหนี้ ณ {0} · เลยกำหนดคำนวณเมื่อมีทั้งจำนวนงวดและวันเก็บ · ยอดลูกหนี้รวมในสินทรัพย์สุทธิ แต่ยังไม่ใช่เงินสด · เงินต้นรับคืนไม่รวมเป็นรายรับใหม่", &[format!("{}", view.today)])} }
            }
        }
    }
}
