use crate::{components::*, state::UiState};
use dioxus::prelude::*;
use ledger_application::*;
use ledger_domain::*;

fn empty_form(month: Month) -> RecurringInput {
    RecurringInput {
        installments: None,
        name: String::new(),
        amount: String::new(),
        day: "1".into(),
        start: month.to_string(),
        account: None,
        category: None,
    }
}

#[component]
pub(crate) fn RecurringPage(view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let current = match Month::of(view.today) {
        Ok(month) => month,
        Err(_) => return rsx! { p { {crate::i18n::text("วันที่ไม่ถูกต้อง", &[])} } },
    };
    let mut month = use_signal(|| current);
    let mut show_form = use_signal(|| false);
    let mut form = use_signal(|| empty_form(current));
    let mut paying = use_signal(|| None::<(RecurringExpense, Month)>);
    let mut stopping = use_signal(|| None::<RecurringExpense>);
    let mut configuring = use_signal(|| None::<RecurringExpense>);
    let mut known_count = use_signal(|| view.recurring.len());
    let mut known_settlements = use_signal(|| view.settlements.clone());
    use_effect(move || {
        if let Some(view) = store.view.read().as_ref() {
            if view.recurring.len() != *known_count.peek() {
                known_count.set(view.recurring.len());
                show_form.set(false);
                form.set(empty_form(current));
            }
            if view.settlements != *known_settlements.peek() {
                known_settlements.set(view.settlements.clone());
                paying.set(None);
            }
            if configuring
                .peek()
                .as_ref()
                .is_some_and(|s| !view.recurring.contains(s))
            {
                configuring.set(None);
            }
            if stopping
                .peek()
                .as_ref()
                .is_some_and(|s| !view.recurring.contains(s))
            {
                stopping.set(None);
            }
        }
    });
    let summary = match recurring_month(&view, month()) {
        Ok(summary) => summary,
        Err(error) => return rsx! { p { role: "alert", "{error}" } },
    };
    let progress = recurring_progress(&view);
    rsx! {
        section { class: "page-heading", div { h1 { {crate::i18n::text("รายจ่ายประจำเดือน", &[])} } p { class: "muted", {crate::i18n::text("วางแผนค่าเช่า ค่าน้ำไฟ ค่าสมาชิก และค่าใช้จ่ายที่ต้องจ่ายทุกเดือน", &[])} } }
            button { class: "primary", disabled: *store.busy.read(), onclick: move |_| show_form.set(!show_form()), {crate::i18n::text("เพิ่มรายจ่ายประจำ", &[])} }
        }
        section { class: "card recurring-card",
            div { class: "recurring-month",
                button { class: "text-button", "aria-label": crate::i18n::text("เดือนก่อน", &[]), onclick: move |_| { if let Ok(previous) = month().shifted(-1) { month.set(previous); } }, "←" }
                label { r#for: "recurring-month", {crate::i18n::text("งวดเดือน (ค.ศ.)", &[])} }
                input { id: "recurring-month", r#type: "month", min: "1900-01", max: "9999-12", value: "{month}", onchange: move |e| { if let Ok(value) = e.value().parse() { month.set(value); } } }
                button { class: "text-button", "aria-label": crate::i18n::text("เดือนถัดไป", &[]), onclick: move |_| { if let Ok(next) = month().shifted(1) { month.set(next); } }, "→" }
            }
            div { class: "recurring-totals",
                div { small { {crate::i18n::text("ยอดตามแผน", &[])} } strong { "{crate::i18n::currency_prefix()}{money_label(summary.planned)}" } }
                div { small { {crate::i18n::text("จ่ายจริงที่ผูกกับงวดนี้", &[])} } strong { "{crate::i18n::currency_prefix()}{money_label(summary.paid)}" } }
                div { small { {crate::i18n::text("ยังค้างจ่ายตามแผน", &[])} } strong { "{crate::i18n::currency_prefix()}{money_label(summary.pending)}" } }
            }
            p { class: "field-hint", {crate::i18n::text("เดือนที่ไม่มีวันที่ 29–31 จะใช้วันสุดท้ายของเดือน · แผนยังไม่หักเงินในบัญชี · กราฟ Flow rate รวมยอดค้างจ่ายนี้เมื่อเปิดโหมดรวมรายจ่ายประจำ", &[])} }
            if summary.items.is_empty() { p { class: "muted", {crate::i18n::text("ยังไม่มีรายจ่ายประจำในเดือนนี้ เพิ่มแผนเพื่อดูวันครบกำหนดและยอดรวม", &[])} } }
            for item in summary.items {
                { let schedule = item.schedule.clone(); let for_stop = schedule.clone(); let period = month();
                  rsx! {
                    article { class: "recurring-item",
                        div { h3 { "{schedule.name().as_str()}" }
                            p { {crate::i18n::text("ครบกำหนด {0} · ทุกวันที่ {1}", &[format!("{}", item.due), format!("{}", schedule.due().day())])} }
                            p { class: "field-hint", "{crate::recurring_picker::installment_label(&schedule, period)}" }
                            p { class: "field-hint", {crate::i18n::text("{0} · {1}", &[crate::i18n::tr(schedule.category().label()).to_string(), schedule.account().map(|id| account_label(&view, id)).unwrap_or_else(|| "กรุณาเลือกบัญชีใหม่เมื่อบันทึกจ่าย".into()).to_string()])} }
                        }
                        div { class: "recurring-item-actions", strong { "{crate::i18n::currency_prefix()}{money_label(schedule.amount().money())}" }
                            if item.paid_entry.is_some() { span { class: "status-pill", {crate::i18n::text("จ่ายแล้ว ฿{0}", &[money_label(item.paid_amount).to_string()])} } }
                            else {
                                span { class: if item.due < view.today { "notice error" } else { "status-pill" }, if item.due < view.today { {crate::i18n::text("เลยกำหนด · ยังไม่บันทึกจ่าย", &[])} } else { {crate::i18n::text("ยังไม่จ่าย", &[])} } }
                                button { class: "primary", disabled: *store.busy.read(), onclick: move |_| { store.recurring_prepared.set(None); paying.set(Some((schedule.clone(), period))); }, {crate::i18n::text("บันทึกการจ่าย / ใช้รายการเดิม", &[])} }
                            }
                            if for_stop.stopped_from().is_none() {
                                button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| stopping.set(Some(for_stop.clone())), {crate::i18n::text("หยุดแผนตั้งแต่งวดนี้", &[])} }
                            }
                        }
                    }
                  }
                }
            }
        }
        section { class: "card recurring-card",
            h2 { {crate::i18n::text("แผนที่ยังจ่ายไม่ครบ", &[])} }
            p { class: "field-hint", {crate::i18n::text("งวดค้างยังอยู่แม้ผ่านเดือนสุดท้ายแล้ว เลือกบันทึกงวดค้างหรือจ่ายล่วงหน้าได้", &[])} }
            if progress.iter().all(|p| p.next_unpaid.is_none()) { p { {crate::i18n::text("ไม่มีแผนที่ต้องจ่ายต่อแล้ว", &[])} } }
            for p in progress.iter().filter(|p| p.next_unpaid.is_some()) {
                { let schedule = p.schedule.clone(); let config = schedule.clone();
                  rsx! { article { class: "recurring-item",
                    div { h3 { "{schedule.name().as_str()}" }
                        if let Some(remaining) = p.remaining { p { {crate::i18n::text("จ่ายแล้ว {0}/{1} งวด · เหลือ {2} งวด", &[format!("{}", p.paid), format!("{}", schedule.due().installments().unwrap_or(0)), format!("{}", remaining)])} } }
                        else { p { {crate::i18n::text("ไม่กำหนดจำนวนงวด · จ่ายแล้ว {0} งวด", &[format!("{}", p.paid)])} } }
                    }
                    div { class: "recurring-item-actions",
                        if let Some(next) = p.next_unpaid {
                            p { "{crate::recurring_picker::installment_label(&schedule, next)}" }
                            button { class: "primary", disabled: *store.busy.read(), onclick: move |_| { store.recurring_prepared.set(None); paying.set(Some((schedule.clone(), next))); }, {crate::i18n::text("บันทึกงวดที่ยังไม่จ่าย", &[])} }
                        }
                        button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| configuring.set(Some(config.clone())), {crate::i18n::text("ตั้งจำนวนงวด", &[])} }
                    }
                  } }
                }
            }
            details { class: "flow-explanation", summary { {crate::i18n::text("แผนที่จ่ายครบ / หยุดแล้ว", &[])} }
                for p in progress.iter().filter(|p| p.next_unpaid.is_none()) {
                    { let schedule = p.schedule.clone(); rsx! { article { class: "recurring-item",
                        div { strong { "{schedule.name().as_str()}" }
                            if p.completed { p { class: "status-pill", {crate::i18n::text("จ่ายครบ {0} งวดแล้ว", &[format!("{}", p.paid)])} } }
                            else { p { {crate::i18n::text("หยุดแผนแล้ว · จ่ายแล้ว {0} งวด", &[format!("{}", p.paid)])} } }
                        }
                        button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| configuring.set(Some(schedule.clone())), {crate::i18n::text("ตั้งจำนวนงวด", &[])} }
                    } } }
                }
            }
            p { class: "field-hint", {crate::i18n::text("เปลี่ยนยอดหรือวันครบกำหนดได้โดยหยุดแผนเดิม แล้วเพิ่มแผนใหม่ · การจ่ายยอดหนี้บัญชีบัตรเครดิตยังใช้การโอนเข้าบัญชีบัตร ไม่ลงเป็นรายจ่ายซ้ำ", &[])} }
        }
        if show_form() {
            section { class: "card recurring-card",
                onmounted: move |_| { let _ = document::eval("document.getElementById('rec-name').focus()"); },
                h2 { {crate::i18n::text("เพิ่มแผนรายจ่ายประจำ", &[])} }
                form { onsubmit: move |event| { event.prevent_default(); store.send(Command::AddRecurring(form())); },
                    fieldset { disabled: *store.busy.read(), class: "recurring-fields",
                        div { label { r#for: "rec-name", {crate::i18n::text("ชื่อค่าใช้จ่าย", &[])} } input { id: "rec-name", required: true, maxlength: 60, placeholder: crate::i18n::text("เช่น ค่าเช่าห้อง", &[]), value: form.read().name.clone(), oninput: move |e| form.write().name = e.value() } }
                        div { label { r#for: "rec-amount", {crate::i18n::text("ยอดต่อเดือน (บาท)", &[])} } input { id: "rec-amount", required: true, inputmode: "decimal", placeholder: "8500.00", value: form.read().amount.clone(), oninput: move |e| form.write().amount = e.value() } }
                        div { label { r#for: "rec-day", {crate::i18n::text("ทุกวันที่ของเดือน (1–31)", &[])} } input { id: "rec-day", r#type: "number", min: "1", max: "31", required: true, value: form.read().day.clone(), oninput: move |e| form.write().day = e.value() } }
                        div { label { r#for: "rec-start", {crate::i18n::text("เริ่มงวดเดือน (ค.ศ.)", &[])} } input { id: "rec-start", r#type: "month", min: "1900-01", max: "9999-12", required: true, value: form.read().start.clone(), onchange: move |e| form.write().start = e.value() } }
                        div { class: "installments-field",
                            label { class: "flow-mode", input { r#type: "checkbox", checked: form.read().installments.is_some(), onchange: move |e| form.write().installments = e.checked().then(String::new) } {crate::i18n::text("กำหนดจำนวนงวด", &[])} }
                            if let Some(count) = form.read().installments.clone() {
                                label { r#for: "rec-count", {crate::i18n::text("จำนวนงวดทั้งหมด (นับจากเดือนเริ่ม)", &[])} }
                                input { id: "rec-count", r#type: "number", min: "1", max: "1200", required: true, placeholder: crate::i18n::text("เช่น 12", &[]), value: count, oninput: move |e| form.write().installments = Some(e.value()) }
                            } else { p { class: "field-hint", {crate::i18n::text("ไม่กำหนดจำนวนงวด — เกิดซ้ำทุกเดือนจนกว่าจะหยุดแผน", &[])} } }
                        }
                        div { label { r#for: "rec-account", {crate::i18n::text("บัญชีที่จะใช้จ่าย", &[])} }
                            select { id: "rec-account", required: true, value: form.read().account.map(|id| id.to_string()).unwrap_or_default(), onchange: move |e| form.write().account = e.value().parse().ok(),
                                option { value: "", selected: form.read().account.is_none(), {crate::i18n::text("เลือกบัญชี", &[])} }
                                for a in &view.accounts { if !a.account.is_archived() { option { value: "{a.account.id()}", selected: form.read().account == Some(a.account.id()), "{a.account.name().as_str()}" } } }
                            }
                        }
                        div { label { r#for: "rec-category", {crate::i18n::text("หมวดรายจ่าย", &[])} }
                            select { id: "rec-category", required: true, value: form.read().category.map(|c| c.code()).unwrap_or(""), onchange: move |e| form.write().category = Category::from_code(&e.value()).ok(),
                                option { value: "", selected: form.read().category.is_none(), {crate::i18n::text("เลือกหมวดหมู่", &[])} }
                                for c in Category::EXPENSE { option { value: c.code(), selected: form.read().category == Some(c), "{crate::i18n::tr(c.label())}" } }
                            }
                        }
                    }
                    p { class: "field-hint", {crate::i18n::text("ยอดนี้เป็นแผนรายเดือน ก่อนจ่ายจริงสามารถแก้ยอดและบัญชีได้", &[])} }
                    button { class: "primary", r#type: "submit", disabled: *store.busy.read(), {crate::i18n::text("บันทึกแผนรายเดือน", &[])} }
                    button { class: "text-button", r#type: "button", disabled: *store.busy.read(), onclick: move |_| show_form.set(false), {crate::i18n::text("ยกเลิก", &[])} }
                }
            }
        }
        if let Some(schedule) = configuring() {
            InstallmentsDialog { key: "{schedule.id()}", schedule, onclose: move |_| configuring.set(None) }
        }
        if let Some((schedule, period)) = paying() {
            PaymentDialog { key: "{schedule.id()}-{period}", schedule, month: period, view: view.clone(), onclose: move |_| { paying.set(None); store.recurring_prepared.set(None); } }
        }
        if let Some(schedule) = stopping() {
            dialog { id: "rec-stop-dialog", class: "account-dialog recurring-dialog", "aria-label": crate::i18n::text("ยืนยันหยุดรายจ่ายประจำ", &[]),
                onmounted: move |_| { let _ = document::eval("document.getElementById('rec-stop-dialog').showModal()"); },
                oncancel: move |e| { e.prevent_default(); if !*store.busy.read() { stopping.set(None); } },
                if let Some((true, error)) = store.notice.read().clone() { p { role: "alert", class: "form-error", "{error}" } }
                h2 { {crate::i18n::text("หยุด {0} ตั้งแต่ {1}", &[schedule.name().as_str().to_string(), format!("{}", month)])} }
                p { {crate::i18n::text("ยอดที่ยังไม่จ่ายตั้งแต่งวดนี้จะไม่รวมในแผนและ Flow rate อีก รายจ่ายที่บันทึกแล้วจะยังอยู่ หากเป็นเพียงการจ่ายแล้ว ให้ใช้บันทึกการจ่ายแทน", &[])} }
                button { class: "primary", disabled: *store.busy.read(), onclick: move |_| store.send(Command::StopRecurring { expected: schedule.clone(), month: month() }), {crate::i18n::text("ยืนยันหยุดแผน", &[])} }
                button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| stopping.set(None), {crate::i18n::text("กลับ", &[])} }
            }
        }
    }
}

#[component]
fn PaymentDialog(
    schedule: RecurringExpense,
    month: Month,
    view: Dashboard,
    onclose: EventHandler,
) -> Element {
    let mut store = use_context::<UiState>();
    let mut input = use_signal(|| recurring_payment_input(&schedule, view.today));
    let mut existing = use_signal(String::new);
    let prepared = store.recurring_prepared.read().clone();
    let candidates: Vec<_> = view
        .entries
        .iter()
        .filter(|e| {
            matches!(e.kind(), EntryKind::Expense { .. })
                && !view.reversed.contains(&e.id())
                && !view.settlements.iter().any(|s| s.entry == e.id())
        })
        .collect();
    rsx! {
        dialog { id: "rec-pay-dialog", class: "account-dialog recurring-dialog", "aria-label": crate::i18n::text("บันทึกจ่ายรายจ่ายประจำ", &[]),
            onmounted: move |_| { let _ = document::eval("document.getElementById('rec-pay-dialog').showModal()"); },
            oncancel: move |e| { e.prevent_default(); if !*store.busy.read() { onclose.call(()); } },
            if let Some((true, error)) = store.notice.read().clone() { p { role: "alert", class: "form-error", "{error}" } }
            h2 { "{schedule.name().as_str()}" }
            p { "{crate::recurring_picker::installment_label(&schedule, month)}" }
            if let Some(prepared) = prepared {
                p { {crate::i18n::text("ตรวจยอดจ่ายจริง ฿{0}", &[money_label(entry_label(&prepared.payment.entry).map(|(_, amount, _)| amount).unwrap_or(Money::ZERO)).to_string()])} }
                p { {crate::i18n::text("วันที่ {0} · {1}", &[format!("{}", prepared.payment.entry.date()), input.read().account.map(|id| account_label(&view, id)).unwrap_or_default().to_string()])} }
                p { {crate::i18n::text("จะลงรายจ่ายหนึ่งรายการ และเปลี่ยนงวดนี้เป็นจ่ายแล้ว ยอดแผนจะไม่ถูกบวกซ้ำใน Flow rate", &[])} }
                button { class: "primary", disabled: *store.busy.read(), onclick: move |_| store.send(Command::PayRecurring(prepared.clone())), {crate::i18n::text("ยืนยันจ่ายและบันทึกรายจ่าย", &[])} }
                button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.recurring_prepared.set(None), {crate::i18n::text("กลับไปแก้", &[])} }
            } else {
                label { r#for: "rec-existing", {crate::i18n::text("เคยบันทึกรายจ่ายนี้แล้วหรือยัง", &[])} }
                select { id: "rec-existing", disabled: *store.busy.read(), value: "{existing}", onchange: move |e| existing.set(e.value()),
                    option { value: "", selected: existing().is_empty(), {crate::i18n::text("ยัง — บันทึกรายจ่ายใหม่", &[])} }
                    for entry in candidates { if let Some((_, amount, account)) = entry_label(entry) {
                        option { value: "{entry.id()}", selected: existing() == entry.id().to_string(), "{entry.date()} · {entry.note().as_str()} · {account_label(&view, account)} · {crate::i18n::currency_prefix()}{money_label(amount)}" }
                    } }
                }
                if !existing().is_empty() {
                    p { class: "batch-question", {crate::i18n::text("จะผูกงวดนี้กับรายจ่ายที่เลือก โดยไม่สร้างรายจ่ายเพิ่ม ตรวจยอด วันที่ และบัญชีให้ตรงก่อนยืนยัน", &[])} }
                    button { class: "primary", disabled: *store.busy.read(), onclick: move |_| { if let Ok(entry) = existing().parse() { store.send(Command::LinkRecurring { expected: schedule.clone(), month, entry }); } }, {crate::i18n::text("ยืนยันใช้รายการเดิม", &[])} }
                } else {
                    form { onsubmit: move |e| { e.prevent_default(); store.send(Command::PreviewRecurringPayment { expected: schedule.clone(), month, input: input() }); },
                        fieldset { class: "recurring-fields", disabled: *store.busy.read(),
                            div { label { r#for: "rec-pay-amount", {crate::i18n::text("ยอดจ่ายจริง (บาท)", &[])} } input { id: "rec-pay-amount", required: true, inputmode: "decimal", value: input.read().amount.clone(), oninput: move |e| input.write().amount = e.value() } }
                            div { label { r#for: "rec-pay-date", {crate::i18n::text("วันที่จ่ายจริง", &[])} } input { id: "rec-pay-date", r#type: "date", required: true, min: "1900-01-01", max: "{view.today}", value: input.read().date.clone(), onchange: move |e| input.write().date = e.value() } }
                            div { label { r#for: "rec-pay-account", {crate::i18n::text("บัญชีที่จ่าย", &[])} }
                                select { id: "rec-pay-account", required: true, value: input.read().account.map(|id| id.to_string()).unwrap_or_default(), onchange: move |e| input.write().account = e.value().parse().ok(),
                                    option { value: "", selected: input.read().account.is_none(), {crate::i18n::text("เลือกบัญชี", &[])} }
                                    for a in &view.accounts { if !a.account.is_archived() { option { value: "{a.account.id()}", selected: input.read().account == Some(a.account.id()), "{a.account.name().as_str()}" } } }
                                }
                            }
                        }
                        p { class: "field-hint", {crate::i18n::text("จ่ายล่วงหน้าได้ โดยระบุวันที่จ่ายจริง · หากเคยจดในบันทึกด่วน ให้เลือกใช้รายการเดิมด้านบน", &[])} }
                        button { class: "primary", r#type: "submit", disabled: *store.busy.read(), {crate::i18n::text("ตรวจยอดจ่ายก่อนบันทึก", &[])} }
                    }
                }
            }
            button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| onclose.call(()), {crate::i18n::text("ปิด", &[])} }
        }
    }
}

#[component]
fn InstallmentsDialog(schedule: RecurringExpense, onclose: EventHandler) -> Element {
    let store = use_context::<UiState>();
    let mut count = use_signal(|| schedule.due().installments().map(|n| n.to_string()));
    rsx! {
        dialog { id: "rec-count-dialog", class: "account-dialog", "aria-label": crate::i18n::text("ตั้งจำนวนงวด", &[]),
            onmounted: move |_| { let _ = document::eval("document.getElementById('rec-count-dialog').showModal()"); },
            oncancel: move |e| { e.prevent_default(); if !*store.busy.read() { onclose.call(()); } },
            h2 { {crate::i18n::text("ตั้งจำนวนงวด · {0}", &[schedule.name().as_str().to_string()])} }
            if let Some((true, error)) = store.notice.read().clone() { p { class: "form-error", role: "alert", "{error}" } }
            form { onsubmit: move |e| {
                e.prevent_default();
                if parse_installments(count().as_deref()).is_ok_and(|value| value == schedule.due().installments()) {
                    onclose.call(());
                } else {
                    store.send(Command::SetRecurringInstallments { expected: schedule.clone(), installments: count() });
                }
            },
                label { class: "flow-mode", input { r#type: "checkbox", checked: count.read().is_some(), disabled: *store.busy.read(), onchange: move |e| count.set(e.checked().then(String::new)) } {crate::i18n::text("กำหนดจำนวนงวด", &[])} }
                if let Some(value) = count() {
                    label { r#for: "edit-installments", {crate::i18n::text("จำนวนงวดทั้งหมด รวมงวดที่จ่ายแล้ว", &[])} }
                    input { id: "edit-installments", r#type: "number", min: "1", max: "1200", required: true, value, disabled: *store.busy.read(), oninput: move |e| count.set(Some(e.value())) }
                } else { p { {crate::i18n::text("ไม่กำหนดจำนวนงวด — เกิดซ้ำทุกเดือนจนกว่าจะหยุดแผน", &[])} } }
                p { class: "field-hint", {crate::i18n::text("จำนวนงวดนับจากเดือนเริ่มต้นของแผน จ่ายครบแล้วจะไม่มีงวดถัดไป ประวัติที่เคยจ่ายจะไม่ถูกตัดทิ้ง", &[])} }
                button { class: "primary", r#type: "submit", disabled: *store.busy.read(), {crate::i18n::text("บันทึกจำนวนงวด", &[])} }
                button { class: "text-button", r#type: "button", disabled: *store.busy.read(), onclick: move |_| onclose.call(()), {crate::i18n::text("กลับ", &[])} }
            }
        }
    }
}
