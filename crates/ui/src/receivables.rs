use crate::{components::*, state::*};
use dioxus::prelude::*;
use ledger_application::*;
use ledger_domain::*;

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

#[component]
pub(crate) fn ReceivablesPage(view: Dashboard) -> Element {
    let tab = use_signal(|| 0usize);
    let mut store = use_context::<UiState>();
    let mut adding = use_signal(|| false);
    let mut form = use_signal(|| new_form(view.today));
    let mut is_new_loan = use_signal(|| false);
    let mut known_count = use_signal(|| view.receivables.len());
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
    let summary = match receivable_summary(&view) {
        Ok(value) => value,
        Err(e) => return rsx! { p { role: "alert", "{e}" } },
    };
    let review = store.receivable_review.read().clone();
    rsx! {
        section { class: "page-heading",
            div { h1 { {crate::i18n::text("ลูกหนี้", &[])} } p { class: "muted", {crate::i18n::text("ใครค้างเราเท่าไหร่ รับคืนแล้วกี่งวด และนัดเก็บเงินวันไหน", &[])} } }
            button { class: "primary", "aria-haspopup": "dialog", disabled: *store.busy.read(), onclick: move |_| { adding.set(true); store.notice.set(None); store.receivable_review.set(None); }, {crate::i18n::text("เพิ่มลูกหนี้", &[])} }
        }
        PageTabs { id: "receivables", tabs: vec![("list", "รายการลูกหนี้"), ("up", "ภาพรวมการรับคืน")], selected: tab }
        PagePanel { lazy: true, id: "receivables", index: 1, selected: tab(), ReceivablesChart { view: view.clone() } }
        if adding() {
            dialog { id: "receivable-create-dialog", class: "account-dialog recurring-dialog receivable-dialog", "aria-labelledby": "receivable-create-title",
                onmounted: move |_| { let _ = document::eval("document.getElementById('receivable-create-dialog').showModal(); document.getElementById('debtor-name')?.focus()"); },
                oncancel: move |e| { e.prevent_default(); if !*store.busy.read() { adding.set(false); store.receivable_review.set(None); store.notice.set(None); } },
                div { class: "section-heading",
                    h2 { id: "receivable-create-title", tabindex: "-1", {crate::i18n::text("เพิ่มรายการหนี้ที่คนอื่นติดเรา", &[])} }
                    button { class: "icon-button", disabled: *store.busy.read(), "aria-label": crate::i18n::tr("ปิดหน้าต่าง"), onclick: move |_| { adding.set(false); store.receivable_review.set(None); store.notice.set(None); }, Icon { name: "close", size: 20 } }
                }
                if let Some((true, message)) = store.notice.read().clone() { p { class: "form-error", role: "alert", "{crate::i18n::tr(&message)}" } }
                if let Some(ReceivableReview::New(prepared)) = review {
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
                    form { onsubmit: move |e| {
                        e.prevent_default();
                        if is_new_loan() && form.read().source.is_none() { store.notice.set(Some((true, "กรุณาเลือกบัญชีที่จ่ายเงินให้ยืม".into()))); }
                        else { store.send(Command::PreviewReceivable(form())); }
                    },
                        fieldset { class: "recurring-fields", disabled: *store.busy.read(),
                            div { label { r#for: "debtor-name", {crate::i18n::text("ชื่อลูกหนี้", &[])} } input { id: "debtor-name", required: true, maxlength: 60, value: form.read().debtor.clone(), onmounted: move |_| { let _ = document::eval("document.getElementById('debtor-name')?.focus()"); }, oninput: move |e| form.write().debtor = e.value() } }
                            div { label { r#for: "debt-description", {crate::i18n::text("หนี้อะไร", &[])} } input { id: "debt-description", required: true, maxlength: 300, placeholder: crate::i18n::text("เช่น ยืมซื้อคอมพิวเตอร์", &[]), value: form.read().description.clone(), oninput: move |e| form.write().description = e.value() } }
                            div { label { r#for: "debt-total", {crate::i18n::text("เงินต้นทั้งหมดที่ยังค้าง ณ วันที่ตั้งหนี้ (บาท)", &[])} } input { id: "debt-total", required: true, inputmode: "decimal", value: form.read().total.clone(), oninput: move |e| form.write().total = e.value() } }
                            div { label { r#for: "debt-opened", {crate::i18n::text("วันที่ตั้งยอดหนี้", &[])} } input { id: "debt-opened", r#type: "date", required: true, min: "1900-01-01", max: "{view.today}", value: form.read().opened.clone(), onchange: move |e| form.write().opened = e.value() } }
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
                                label { class: "flow-mode", input { r#type: "checkbox", checked: is_new_loan(), onchange: move |e| { is_new_loan.set(e.checked()); form.write().source = None; } } {crate::i18n::text("เป็นการให้ยืมเงินใหม่ หักเงินจากบัญชี", &[])} }
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
                            button { class: "soft-button", r#type: "button", disabled: *store.busy.read(), onclick: move |_| { adding.set(false); store.receivable_review.set(None); store.notice.set(None); }, {crate::i18n::tr("ยกเลิก")} }
                        }
                    }
                }
            }
        }
        PagePanel { lazy: true, id: "receivables", index: 0, selected: tab(), section { class: "card recurring-card",
            h2 { {crate::i18n::text("รายการลูกหนี้และการชำระ", &[])} }
            if summary.items.is_empty() { p { class: "muted", {crate::i18n::text("ยังไม่มีลูกหนี้ เพิ่มรายการแรกเพื่อเริ่มติดตามยอดค้าง", &[])} } }
            for p in summary.items {
                article { class: "debtor-item", key: "{p.loan.id()}",
                    div { class: "section-heading", h3 { "{p.loan.debtor().as_str()}" } span { class: if p.status == ReceivableStatus::Overdue { "status-pill debt-overdue" } else { "status-pill" }, "{crate::i18n::tr(p.status.label())}" } }
                    p { "{p.loan.description().as_str()}" }
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
                    if p.outstanding > Money::ZERO {
                        button { class: "primary", disabled: *store.busy.read(), onclick: { let id = p.loan.id(); move |_| { store.receivable_review.set(None); store.repayment_selection.set(Some(id)); store.repayment_form.set(true); } }, Icon { name: "down", size: 18 } {crate::i18n::text("บันทึกลูกหนี้ชำระหนี้", &[])} }
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
