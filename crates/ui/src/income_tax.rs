use crate::{components::money_label, i18n::text, state::UiState};
use dioxus::prelude::*;
use ledger_application::{EntryInput, IncomeTaxInput};
use ledger_domain::{IncomeSection, IncomeTax};

const SECTIONS: [&str; 8] = [
    "ม.40(1) เงินเดือน ค่าจ้าง โบนัส",
    "ม.40(2) ค่ารับทำงาน ค่าธรรมเนียม ค่านายหน้า",
    "ม.40(3) ค่าสิทธิที่หักค่าใช้จ่ายจริงได้",
    "ม.40(4) ดอกเบี้ยที่เลือกนำมารวมคำนวณ",
    "ม.40(5) ค่าเช่าทรัพย์สิน",
    "ม.40(6) วิชาชีพอิสระ",
    "ม.40(7) รับเหมาพร้อมวัสดุสำคัญ",
    "ม.40(8) ธุรกิจและเงินได้อื่น",
];

#[component]
pub(crate) fn IncomeTaxFields(
    input: EntryInput,
    id: String,
    onchange: EventHandler<EntryInput>,
) -> Element {
    let store = use_context::<UiState>();
    let toggle_input = input.clone();
    let section_input = input.clone();
    rsx! {
        section { class: "income-tax-fields",
            label { class: "tax-check", input { id: "{id}-enabled", r#type: "checkbox", checked: input.income_tax.is_some(), disabled: *store.busy.read(), onchange: move |event| {
                let mut updated = toggle_input.clone(); updated.income_tax = event.checked().then(|| Box::new(IncomeTaxInput::default())); onchange.call(updated);
            } } {text("นำรายรับนี้ไปคำนวณภาษีไทย", &[])} }
            if let Some(tax) = &input.income_tax {
                label { r#for: "{id}-section", {text("ประเภทเงินได้ตามมาตรา 40", &[])} }
                select { id: "{id}-section", required: true, disabled: *store.busy.read(), value: tax.section.map(|s| s.number().to_string()).unwrap_or_default(), onchange: move |event| {
                    let mut updated = section_input.clone();
                    if let Some(tax) = updated.income_tax.as_mut() { tax.section = event.value().parse().ok().and_then(|n| IncomeSection::new(n).ok()); }
                    onchange.call(updated);
                },
                    option { value: "", selected: tax.section.is_none(), {text("เลือกประเภทเงินได้ตามหลักฐาน", &[])} }
                    for (index, label) in SECTIONS.iter().enumerate() { option { value: "{index + 1}", selected: tax.section.is_some_and(|s| s.index() == index), {crate::i18n::tr(label)} } }
                }
                div { class: "income-tax-grid",
                    for (index, label, value) in [(0, "เงินได้ก่อนหัก ไม่รวม VAT", tax.gross.clone()), (1, "ภาษีหัก ณ ที่จ่าย", tax.withholding.clone()), (2, "VAT ที่ได้รับรวมมาด้วย", tax.vat.clone()), (3, "รายการหักอื่น เช่น ประกันสังคม", tax.other_deductions.clone())] {
                        { let original = input.clone(); rsx! { div {
                            label { r#for: "{id}-value-{index}", {text(label, &[])} }
                            input { id: "{id}-value-{index}", inputmode: "decimal", required: true, maxlength: 18, value, disabled: *store.busy.read(), oninput: move |event| {
                                let mut updated = original.clone();
                                if let Some(tax) = updated.income_tax.as_mut() { match index { 0 => tax.gross = event.value(), 1 => tax.withholding = event.value(), 2 => tax.vat = event.value(), _ => tax.other_deductions = event.value() } }
                                onchange.call(updated);
                            } }
                        } } }
                    }
                }
                p { class: "field-hint", {text("ยอดรายการด้านบนคือเงินรับจริง: เงินได้ก่อนหัก + VAT − หัก ณ ที่จ่าย − รายการหักอื่น ต้องตรงกัน", &[])} }
                p { class: "field-hint", {text("ใช้วันที่รายการเป็นวันที่ได้รับเงิน · ไม่ดึง VAT เป็นเงินได้ และไม่ใช้รายการหักอื่นเป็นค่าลดหย่อนอัตโนมัติ", &[])} }
                p { class: "field-hint", {text("เลือกเฉพาะเงินได้ที่นำมารวมคำนวณ วิธีเหมา เงินปันผลพร้อมเครดิต และกรณีพิเศษยังต้องตรวจในหน้าภาษี", &[])} }
            }
        }
    }
}

#[component]
pub(crate) fn IncomeTaxSummary(tax: IncomeTax) -> Element {
    rsx! { details { class: "saved-entry-details tax-entry-summary", summary { {text("ภาษี · ม.40({0})", &[tax.section().number().to_string()])} }
        dl {
            for (label, amount) in [("เงินได้ก่อนหัก ไม่รวม VAT", tax.gross()), ("ภาษีหัก ณ ที่จ่าย", tax.withholding()), ("VAT ที่ได้รับรวมมาด้วย", tax.vat()), ("รายการหักอื่น เช่น ประกันสังคม", tax.other_deductions())] {
                div { dt { {text(label, &[])} } dd { "THB {money_label(amount)}" } }
            }
        }
    } }
}
