use crate::{
    components::*,
    i18n::{text, tr},
    state::{Page, UiState},
};
use dioxus::prelude::*;
use ledger_application::*;
use ledger_domain::*;

pub(crate) fn percentage(done: i64, total: i64) -> f64 {
    if total <= 0 {
        0.0
    } else {
        (done.max(0) as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
    }
}

#[component]
pub(crate) fn ProgressRing(done: i64, total: i64, label: String) -> Element {
    let percent = percentage(done, total);
    let display = if total > 0 {
        format!("{percent:.0}%")
    } else {
        "—".into()
    };
    rsx! { div { class: "debt-ring", role: "img", "aria-label": "{label}: {display}",
        style: "--progress:{percent}%;",
        div { strong { "{display}" } span { "{label}" } }
    } }
}

#[component]
pub(crate) fn AmountBar(label: String, amount: Money, maximum: i64, tone: &'static str) -> Element {
    let width = percentage(amount.minor(), maximum);
    rsx! { div { class: "debt-amount-bar", "data-tone": tone,
        div { span { "{label}" } strong { "{crate::i18n::currency_prefix()}{money_label(amount)}" } }
        div { class: "debt-bar-track", "aria-hidden": "true", span { style: "width:{width}%;" } }
    } }
}

#[component]
pub(crate) fn FinancialPosition(view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let paid = match paid_out_this_month(&view) {
        Ok(v) => v,
        Err(e) => return rsx! { p { role: "alert", "{e}" } },
    };
    let month = match Month::of(view.today).and_then(|m| recurring_month(&view, m)) {
        Ok(v) => v,
        Err(e) => return rsx! { p { role: "alert", "{e}" } },
    };
    let max = [
        view.assets.minor(),
        view.liabilities.minor(),
        paid.minor(),
        month.pending.minor(),
    ]
    .into_iter()
    .max()
    .unwrap_or(0);
    rsx! { section { class: "card financial-position",
        div { class: "section-heading", div { span { class: "debt-eyebrow", {text("เงินของเรา ณ วันนี้", &[])} } h2 { {text("มีเท่าไหร่ จ่ายแล้วเท่าไหร่", &[])} } } button { class: "text-button", onclick: move |_| store.page.set(Page::Recurring), {text("ดูหนี้และบิล", &[])} Icon { name: "arrow", size: 20 } } }
        div { class: "financial-position-grid",
            div { class: "position-net", small { {text("สินทรัพย์สุทธิ", &[])} }
                strong { "{crate::i18n::currency_prefix()}{money_label(view.net_worth)}" }
                p { {text("สินทรัพย์ปัจจุบัน − หนี้คงค้าง", &[])} }
                span { class: "status-pill", {text("ลูกหนี้รวมในสินทรัพย์ แต่ยังไม่ใช่เงินสด", &[])} }
            }
            div { class: "position-bars",
                AmountBar { label: text("สินทรัพย์ปัจจุบัน", &[]), amount: view.assets, maximum: max, tone: "asset" }
                AmountBar { label: text("จ่ายเงินจริงเดือนนี้", &[]), amount: paid, maximum: max, tone: "paid" }
                AmountBar { label: text("หนี้คงค้างรอชำระ", &[]), amount: view.liabilities, maximum: max, tone: "due" }
                AmountBar { label: text("แผนเดือนนี้ที่ยังไม่บันทึกจ่าย", &[]), amount: month.pending, maximum: max, tone: "plan" }
            }
        }
        p { class: "field-hint", {text("จ่ายจริง = รายจ่ายจากบัญชีที่ไม่ใช่บัตร + เงินโอนชำระบัตรในเดือนนี้ · ยอดนี้หักจากบัญชีแล้ว จึงไม่หักจากสินทรัพย์ซ้ำ · แผนรายเดือนยังไม่ใช่หนี้ที่บันทึก", &[])} }
    } }
}

pub(crate) fn cycle_from_fields(
    closing: &str,
    payment: &str,
) -> Result<CreditCardCycle, DomainError> {
    CreditCardCycle::new(
        closing
            .parse()
            .map_err(|_| DomainError::InvalidCreditCycle)?,
        payment
            .parse()
            .map_err(|_| DomainError::InvalidCreditCycle)?,
    )
}

#[component]
pub(crate) fn CycleFields(
    closing: Signal<String>,
    payment: Signal<String>,
    prefix: String,
) -> Element {
    rsx! { div { class: "credit-cycle-fields",
        div { label { r#for: "{prefix}-closing", {text("วันตัดรอบบิล", &[])} }
            input { id: "{prefix}-closing", r#type: "number", min: 1, max: 31, required: true, placeholder: "20", value: closing(), oninput: move |e| closing.set(e.value()) }
        }
        div { label { r#for: "{prefix}-payment", {text("วันครบกำหนดชำระ", &[])} }
            input { id: "{prefix}-payment", r#type: "number", min: 1, max: 31, required: true, placeholder: "5", value: payment(), oninput: move |e| payment.set(e.value()) }
        }
        p { class: "field-hint", {text("ระบุวันที่ 1–31 · รูดวันตัดรอบนับในบิลนั้น · วันชำระที่ไม่หลังวันตัดรอบจะเป็นเดือนถัดไป · เดือนสั้นใช้วันสุดท้าย", &[])} }
    } }
}

#[component]
fn LegacyCycleForm(account: Account) -> Element {
    let mut store = use_context::<UiState>();
    let closing = use_signal(String::new);
    let payment = use_signal(String::new);
    let prefix = format!("legacy-{}", account.id());
    rsx! { form { class: "legacy-cycle-form", onsubmit: move |e| {
        e.prevent_default();
        match cycle_from_fields(&closing(), &payment()) {
            Ok(cycle) => store.send(Command::SetCreditCycle { expected: account.clone(), cycle }),
            Err(e) => store.notice.set(Some((true, e.to_string()))),
        }
    },
        p { {text("บัตรเดิมยังไม่มีวันตัดรอบ เติมข้อมูลเพื่อจัดบิลจากรายการที่บันทึกไว้", &[])} }
        fieldset { class: "credit-cycle-group", disabled: *store.busy.read(), "aria-label": tr("บัตรเครดิต"), CycleFields { closing, payment, prefix }
            button { class: "soft-button", r#type: "submit", {text("บันทึกรอบบัตร", &[])} }
        }
    } }
}

#[component]
pub(crate) fn CreditCardsPanel(view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let cards = match credit_cards(&view) {
        Ok(v) => v,
        Err(e) => return rsx! { p { role: "alert", "{e}" } },
    };
    if cards.is_empty() {
        return rsx! {};
    }
    let today = view.today;
    rsx! { section { class: "credit-section",
        div { class: "section-heading", div { span { class: "debt-eyebrow", {text("จ่ายทีหลัง เห็นยอดตั้งแต่วันนี้", &[])} } h2 { {text("บัตรเครดิต · รอชำระ", &[])} } } Icon { name: "wallet", size: 26 } }
        div { class: "credit-card-grid",
            for card in cards {
                { let id = card.account.id(); let name = card.account.name().as_str().to_owned(); let amount = card.outstanding;
                  rsx! { article { class: "card credit-obligation", key: "{id}",
                    div { class: "credit-card-overview",
                    div { class: "credit-card-heading", span { class: "credit-chip", Icon { name: "wallet", size: 22 } } h3 { "{name}" } span { class: "status-pill", {tr("บัตรเครดิต")} } }
                    small { {text("ยอดรอชำระทั้งหมด", &[])} }
                    strong { class: "credit-outstanding", "{crate::i18n::currency_prefix()}{money_label(amount)}" }
                    if let Some(cycle) = card.account.credit_cycle() {
                        div { class: "credit-cycle-labels",
                            span { Icon { name: "calendar", size: 16 } {text("ตัดรอบวันที่ {0}", &[cycle.closing_day().to_string()])} }
                            span { {text("ชำระวันที่ {0}", &[cycle.payment_day().to_string()])} }
                        }
                    } else { LegacyCycleForm { account: card.account.clone() } }
                    if card.prepaid > Money::ZERO { p { class: "status-pill", {text("ยอดจ่ายเกิน / เครดิตคงเหลือ {0}", &[money_label(card.prepaid)])} } }
                    }
                    div { class: "credit-bills",
                    if amount == Money::ZERO { p { class: "debt-clear", Icon { name: "check", size: 22 } {text("ไม่มีหนี้ค้างชำระ", &[])} } }
                    for bill in card.bills.iter().filter(|b| b.outstanding > Money::ZERO) {
                        div { class: "credit-bill", "data-overdue": bill.dates.is_some_and(|d| d.due < today),
                            div {
                                if let Some(dates) = bill.dates {
                                    span { class: "debt-eyebrow", if dates.closing > today { {text("รอบที่ยังไม่ตัดบิล", &[])} } else if dates.due < today { {text("เลยกำหนดชำระ", &[])} } else { {text("รอชำระตามรอบ", &[])} } }
                                    strong { {text("ชำระภายใน {0}", &[dates.due.to_string()])} }
                                    small { {text("ตัดรอบ {0}", &[dates.closing.to_string()])} }
                                } else { strong { {text("ยอดยกมา / ยังไม่ทราบรอบ", &[])} } small { {text("รวมในยอดหนี้แล้ว แต่ยังไม่ระบุว่าเลยกำหนด", &[])} } }
                            }
                            strong { "{crate::i18n::currency_prefix()}{money_label(bill.outstanding)}" }
                        }
                    }
                    if amount > Money::ZERO {
                        button { class: "primary full-width", disabled: *store.busy.read(), onclick: move |_| {
                            store.new_entry();
                            store.input.set(Some(EntryInput { kind: TransactionKind::Transfer, amount: amount.to_string(), destination: Some(id), note: format!("ชำระบัตร {name}"), ..EntryInput::empty(today) }));
                            store.page.set(Page::Manual);
                        }, Icon { name: "arrow", size: 18 } {text("ชำระบัตรจากบัญชี", &[])} }
                    }
                    }
                  } }
                }
            }
        }
        p { class: "field-hint", {text("จัดรอบจากวันที่รายการที่กรอก · จ่ายคืนตัดยอดเก่าก่อน ไม่ลงรายจ่ายซ้ำ · ไม่คำนวณดอกเบี้ยหรือยอดขั้นต่ำของธนาคาร กรุณาเทียบใบแจ้งยอดจริง", &[])} }
    } }
}
