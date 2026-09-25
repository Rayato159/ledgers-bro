use crate::{components::*, i18n::tr};
use dioxus::prelude::*;
use ledger_application::*;
use ledger_domain::*;

#[component]
pub fn EntryBreakdown(view: Dashboard, entry: JournalEntry) -> Element {
    match credit_payment_breakdown(&view, entry.id()) {
        Err(error) => rsx! { p { role: "alert", "{error}" } },
        Ok(Some(payment)) => rsx! {
            section { class: "entry-breakdown",
                h3 { {tr("ชำระหนี้บัตร")} ": {account_label(&view, payment.card)}" }
                p { class: "field-hint", {tr("แยกยอดชำระตามรายการเก่าสุด ณ วันที่จ่าย เป็นการจัดสรรของแอป กรุณาเทียบใบแจ้งยอดธนาคาร")} }
                for (source, allocated) in payment.items {
                    article { class: "charge-detail",
                        div { strong { {charge_title(&source)} } small { "{source.date()}" } }
                        strong { "{crate::i18n::currency_prefix()}{money_label(allocated)}" }
                        if matches!(source.kind(), EntryKind::Opening { .. }) { p { class: "field-hint", {tr("ยอดยกมาบันทึกเป็นยอดรวม ไม่มีรายการย่อยก่อนเริ่มใช้แอป")} } }
                        else if source.note().as_str().contains('\n') { details { class: "charge-source", summary { {tr("ดูรายละเอียด")} } p { class: "entry-note-body", "{source.note().as_str()}" } } }
                    }
                }
                if payment.prepaid > Money::ZERO { p { {tr("ยอดจ่ายล่วงหน้า")} ": {crate::i18n::currency_prefix()}{money_label(payment.prepaid)}" } }
            }
        },
        Ok(None) => rsx! {
            if matches!(entry.kind(), EntryKind::Expense { category: Category::OtherExpense, .. }) && !entry.note().as_str().contains('\n') {
                p { class: "field-hint", {tr("รายการนี้บันทึกเป็นยอดเดียว ไม่มีรายการย่อยหรือการผูกชำระบัตร ใช้เมนูชำระบัตรเพื่อเชื่อมกับหนี้บัตรในครั้งถัดไป")} }
            }
        },
    }
}

fn charge_title(entry: &JournalEntry) -> String {
    if matches!(entry.kind(), EntryKind::Opening { .. }) {
        return tr("ยอดยกมา");
    }
    if !entry.note().as_str().is_empty() {
        entry
            .note()
            .as_str()
            .lines()
            .next()
            .unwrap_or_default()
            .into()
    } else {
        entry_label(entry)
            .map(|(label, _, _)| tr(label))
            .unwrap_or_else(|| tr("รายการ"))
    }
}

#[component]
pub fn CreditChargeDetails(charges: Vec<CreditCharge>) -> Element {
    rsx! {
        div { class: "entry-breakdown",
            h3 { {tr("รายการที่เป็นหนี้คงค้าง")} }
            for charge in charges.iter().filter(|c| c.outstanding > Money::ZERO) {
                article { class: "charge-detail",
                    div { strong { {charge_title(&charge.entry)} } small { "{charge.entry.date()}" } }
                    strong { "{crate::i18n::currency_prefix()}{money_label(charge.outstanding)}" }
                    p { class: "field-hint", {tr("ยอดเดิม")} ": {money_label(charge.charged)} · " {tr("จ่ายแล้ว")} ": {money_label(charge.paid)}" }
                    if matches!(charge.entry.kind(), EntryKind::Opening { .. }) { p { class: "field-hint", {tr("ยอดยกมาบันทึกเป็นยอดรวม ไม่มีรายการย่อยก่อนเริ่มใช้แอป")} } }
                    else if charge.entry.note().as_str().contains('\n') { details { class: "charge-source", summary { {tr("ดูรายละเอียด")} } p { class: "entry-note-body", "{charge.entry.note().as_str()}" } } }
                }
            }
        }
    }
}
