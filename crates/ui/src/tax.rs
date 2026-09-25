use crate::components::money_label;
use dioxus::prelude::*;
use ledger_application::{
    TAX_RULE_VERSION, TAX_SOURCES, TaxWorksheet, annual_tax_income, calculate_tax,
    tax_worksheet_with_entries,
};

#[derive(Clone, Default)]
pub(crate) struct TaxSession {
    pub worksheet: TaxWorksheet,
    pub submitted: bool,
}

fn amount_field(input: &mut TaxWorksheet, group: u8, index: usize) -> &mut String {
    match group {
        0 => &mut input.incomes[index],
        1 => &mut input.expenses[index],
        _ => match index {
            0 => &mut input.life_insurance,
            1 => &mut input.health_insurance,
            2 => &mut input.parent_health,
            3 => &mut input.mortgage_interest,
            4 => &mut input.social_security,
            5 => &mut input.donation,
            6 => &mut input.withholding,
            _ => &mut input.prepayments,
        },
    }
}

#[component]
fn TaxAmount(label: String, group: u8, index: usize) -> Element {
    let mut session = use_context::<Signal<TaxSession>>();
    let value = amount_field(&mut session.read().worksheet.clone(), group, index).clone();
    rsx! {
        div { class: "tax-field",
            label { r#for: "tax-{group}-{index}", "{crate::i18n::tr(&label)}" }
            input { id: "tax-{group}-{index}", inputmode: "decimal", maxlength: 18, required: true, value,
                oninput: move |event| { let mut state = session.write(); *amount_field(&mut state.worksheet, group, index) = event.value(); state.submitted = false; }
            }
        }
    }
}

#[component]
pub(crate) fn TaxPage() -> Element {
    let mut session = use_context::<Signal<TaxSession>>();
    let store = use_context::<crate::state::UiState>();
    let state = session.read().clone();
    let automatic = store
        .view
        .read()
        .as_ref()
        .map(|view| annual_tax_income(view, state.worksheet.year))
        .unwrap_or_else(|| Ok(Default::default()));
    let result = state.submitted.then(|| {
        automatic
            .clone()
            .and_then(|automatic| tax_worksheet_with_entries(&state.worksheet, &automatic))
            .and_then(|worksheet| calculate_tax(&worksheet))
    });
    rsx! {
        section { class: "page-heading", div { h1 { {crate::i18n::text("คำนวณภาษีบุคคลธรรมดา", &[])} } p { class: "muted", {crate::i18n::text("คำนวณรายปีจากข้อมูลและสิทธิที่ยืนยัน · ยังไม่ได้ยื่นแบบ", &[])} } } }
        div { class: "tax-calculator",
            form { class: "card tax-inputs", onsubmit: move |event| {
                event.prevent_default(); session.write().submitted = true;
                let _ = document::eval("requestAnimationFrame(() => { const result = document.getElementById('tax-result'); result.focus({preventScroll: true}); result.scrollIntoView({block: 'start'}); })");
            },
                label { r#for: "tax-year", {crate::i18n::text("ปีภาษี (ปีที่ได้รับเงิน)", &[])} }
                select { id: "tax-year", value: state.worksheet.year.to_string(), onchange: move |event| {
                    if let Ok(year) = event.value().parse() { let mut state = session.write(); state.worksheet.year = year; state.worksheet.eligibility_confirmed = false; state.submitted = false; }
                }, option { value: "2569", {crate::i18n::text("2569 · ประมาณการปีปัจจุบัน", &[])} } option { value: "2568", if crate::i18n::english() { "2025" } else { "2568" } } }
                p { class: "field-hint", {crate::i18n::text("ทุกช่องเป็นยอดรวมทั้งปี หน่วยบาท กรอก 0 ถ้าไม่มี ข้อมูลค้างไว้ระหว่างเปลี่ยนหน้า แต่ยังไม่บันทึกแบบภาษีเมื่อปิดแอป", &[])} }
                section { class: "tax-auto-summary",
                    h2 { {crate::i18n::text("รายได้จากรายการที่เลือกไว้", &[])} }
                    match &automatic {
                        Ok(auto) => rsx! {
                            p { {crate::i18n::text("รวมอัตโนมัติ {0} รายการในปีที่เลือก ไม่รวมรายการยกเลิก", &[auto.count.to_string()])} }
                            dl { for (index, amount) in auto.incomes.iter().enumerate() {
                                div { dt { "ม.40({index + 1})" } dd { "THB {money_label(*amount)}" } }
                            } div { dt { {crate::i18n::text("ภาษีหัก ณ ที่จ่าย", &[])} } dd { "THB {money_label(auto.withholding)}" } } }
                        },
                        Err(error) => rsx! { p { role: "alert", "{error}" } },
                    }
                    p { class: "field-hint", {crate::i18n::text("ยอดจากรายการจะรวมในผลคำนวณให้แล้ว ช่องรายได้และหัก ณ ที่จ่ายด้านล่างให้กรอกเฉพาะยอดเพิ่มเติมที่ยังไม่ได้บันทึก เพื่อไม่ให้นับซ้ำ", &[])} }
                }
                h2 { {crate::i18n::text("1. เงินได้เพิ่มเติมที่ยังไม่อยู่ในรายการ", &[])} }
                p { {crate::i18n::text("ใช้หนังสือ 50 ทวิและหลักฐานรายได้ ไม่ใช้ยอดสุทธิที่โอนเข้าธนาคารแทน และไม่รวม VAT เป็นรายได้", &[])} }
                TaxAmount { label: crate::i18n::text("ม.40(1) เงินเดือน ค่าจ้าง โบนัส", &[]), group: 0, index: 0 }
                TaxAmount { label: crate::i18n::text("ม.40(2) ค่ารับทำงาน ค่าธรรมเนียม ค่านายหน้า", &[]), group: 0, index: 1 }
                p { class: "field-hint", {crate::i18n::text("สองประเภทรวมกันหักค่าใช้จ่าย 50% ไม่เกิน 100,000 บาท ค่าจ้างงานบางแบบเป็น ม.40(8) ต้องตรวจตามข้อเท็จจริง", &[])} }
                details { class: "tax-extra", summary { {crate::i18n::text("เงินได้ประเภทอื่น ม.40(3)–(8)", &[])} }
                    for (index, label) in [(2, "ม.40(3) ค่าสิทธิที่หักค่าใช้จ่ายจริงได้"), (3, "ม.40(4) ดอกเบี้ยที่เลือกนำมารวมคำนวณ"), (4, "ม.40(5) ค่าเช่าทรัพย์สิน"), (5, "ม.40(6) วิชาชีพอิสระ"), (6, "ม.40(7) รับเหมาพร้อมวัสดุสำคัญ"), (7, "ม.40(8) ธุรกิจและเงินได้อื่น")] {
                        TaxAmount { label: label.to_owned(), group: 0, index }
                        if index != 3 { TaxAmount { label: crate::i18n::text("ค่าใช้จ่ายจริงที่มีสิทธิหัก ม.40({0})", &[(index + 1).to_string()]), group: 1, index } }
                    }
                    p { class: "field-hint", {crate::i18n::text("ค่าใช้จ่ายจริงต้องจำเป็น สมควร เกี่ยวกับเงินได้นั้น และมีหลักฐาน รุ่นนี้ยังไม่เปรียบเทียบวิธีเหมาตามประเภทย่อยให้ ถ้าต้องการวิธีเหมาให้เลือกกรณีที่ยังไม่รองรับด้านล่าง", &[])} }
                }
                h2 { {crate::i18n::text("2. ค่าลดหย่อนที่มีสิทธิ", &[])} }
                p { {crate::i18n::text("ส่วนตัว 60,000 บาท คำนวณให้อัตโนมัติ", &[])} }
                label { class: "tax-check", input { r#type: "checkbox", checked: state.worksheet.spouse_no_income, onchange: move |event| { let mut state = session.write(); state.worksheet.spouse_no_income = event.checked(); state.submitted = false; } } {crate::i18n::text("คู่สมรสจดทะเบียนไม่มีเงินได้และเข้าเงื่อนไขสิทธิ 60,000 บาท", &[])} }
                details { class: "tax-extra", summary { {crate::i18n::text("บุตร บิดามารดา และผู้พิการ", &[])} }
                    for (index, label, value) in [(0, "บุตรที่มีสิทธิ (คน)", state.worksheet.eligible_children), (1, "ในจำนวนข้างต้น: บุตรชอบด้วยกฎหมายคนที่ 2 เป็นต้นไป เกิดตั้งแต่ 2561 (คน)", state.worksheet.additional_children), (2, "บิดามารดาที่มีสิทธิ ไม่ใช้ซ้ำกับผู้อื่น (คน)", state.worksheet.eligible_parents), (3, "ผู้พิการ/ทุพพลภาพที่มีสิทธิอุปการะ (คน)", state.worksheet.eligible_disabled)] {
                        label { r#for: "tax-count-{index}", "{crate::i18n::tr(&label)}" }
                        input { id: "tax-count-{index}", r#type: "number", min: 0, max: 100, step: 1, value: value.to_string(), required: true, onchange: move |event| { if let Ok(value) = event.value().parse() { let mut state = session.write(); match index { 0 => state.worksheet.eligible_children = value, 1 => state.worksheet.additional_children = value, 2 => state.worksheet.eligible_parents = value, _ => state.worksheet.eligible_disabled = value }; state.submitted = false; } } }
                    }
                    p { class: "field-hint", {crate::i18n::text("บุตรต้องเข้าเงื่อนไขอายุ/การศึกษา/เงินได้; บุตรบุญธรรมมีเพดานจำนวนและไม่ได้สิทธิเพิ่มเติม บิดามารดาต้องอายุ 60 ปีขึ้นไป เงินได้ไม่เกิน 30,000 บาท และมี ลย.03 ผู้พิการต้องมีหลักฐานและสิทธิอุปการะตามเกณฑ์กรมสรรพากร", &[])} }
                }
                for (index, label) in [(0, "เบี้ยประกันชีวิตตนเองที่เข้าเงื่อนไข"), (1, "เบี้ยประกันสุขภาพตนเองที่เข้าเงื่อนไข"), (2, "ประกันสุขภาพบิดามารดา เฉพาะส่วนสิทธิของตน"), (3, "ดอกเบี้ยบ้าน เฉพาะส่วนสิทธิของตน"), (4, "ประกันสังคม ม.33 ของตนเอง ตามจ่ายจริง"), (5, "บริจาคทั่วไปที่มีสิทธิหัก 1 เท่า")] { TaxAmount { label: label.to_owned(), group: 2, index } }
                p { class: "field-hint", {crate::i18n::text("ประกันชีวิตต้องเข้าเงื่อนไขสัญญา 10 ปีขึ้นไปและแจ้งใช้สิทธิ ประกันสุขภาพไม่เกิน 25,000 บาท รวมประกันชีวิตไม่เกิน 100,000 บาท บริจาคทั่วไปไม่เกิน 10% หลังลดหย่อน; กรณี e-Donation ให้ตรวจหลักฐานของปีที่ใช้สิทธิ", &[])} }
                h2 { {crate::i18n::text("3. ภาษีที่ชำระไว้แล้ว", &[])} }
                TaxAmount { label: crate::i18n::text("หัก ณ ที่จ่ายเพิ่มเติมที่ยังไม่อยู่ในรายการ", &[]), group: 2, index: 6 }
                TaxAmount { label: crate::i18n::text("ภาษีครึ่งปี/ชำระล่วงหน้าที่นำมาเครดิตได้", &[]), group: 2, index: 7 }
                details { class: "tax-extra", open: true, summary { {crate::i18n::text("ตรวจขอบเขตก่อนคำนวณ", &[])} }
                    p { {crate::i18n::text("รองรับการคำนวณรายปีของบุคคลธรรมดาผู้อยู่ในไทย ยื่นแยกของตนเอง ไม่ใช่ VAT หรือภาษีธุรกิจทุกประเภท", &[])} }
                    label { class: "tax-check", input { r#type: "checkbox", checked: state.worksheet.unsupported_items, onchange: move |event| { let mut state = session.write(); state.worksheet.unsupported_items = event.checked(); state.submitted = false; } }
                        {crate::i18n::text("มีกรณีที่ยังไม่รองรับ: RMF/กองทุนเกษียณ/ประกันบำนาญ/Thai ESG, สิทธิพิเศษรายปี, ประกันสังคม ม.39/40, ประกันชีวิตคู่สมรส, เงินบริจาค 2 เท่า, ยกเว้นผู้สูงอายุ/ผู้พิการ, เงินได้ต่างประเทศ/เครดิตภาษีต่างประเทศ, เงินปันผลพร้อมเครดิต, เงินออกจากงานแยกคำนวณ, วิธีเหมา ม.40(3)–(8), ภาษีอัตราพิเศษ หรือยื่นรวมคู่สมรส", &[])}
                    }
                    label { class: "tax-check", input { r#type: "checkbox", checked: state.worksheet.eligibility_confirmed, onchange: move |event| { let mut state = session.write(); state.worksheet.eligibility_confirmed = event.checked(); state.submitted = false; } } {crate::i18n::text("ตรวจแล้วว่าข้อมูลเงินได้ครบ ค่าใช้จ่ายมีหลักฐาน และจำนวนสิทธิลดหย่อนที่กรอกเข้าเงื่อนไขของปีนี้ ไม่ใช้สิทธิซ้ำ", &[])} }
                }
                button { class: "primary full-width", r#type: "submit", {crate::i18n::text("คำนวณภาษีตามข้อมูลที่กรอก", &[])} }
            }
            section { class: "card tax-result", id: "tax-result", tabindex: "-1", "aria-live": "polite",
                h2 { {crate::i18n::text("ผลคำนวณและที่มา", &[])} }
                if let Some(result) = result {
                    match result {
                        Err(error) => rsx! { p { class: "form-error", role: "alert", "{error}" } },
                        Ok(result) => rsx! {
                            span { class: "status-pill", {crate::i18n::text("ปี {0} · ผลตามข้อมูลที่ยืนยัน", &[format!("{}", result.year)])} }
                            div { class: "tax-total", small { {crate::i18n::text("ยอดต้องชำระเพิ่ม", &[])} } strong { "{crate::i18n::currency_prefix()}{money_label(result.payable)}" } }
                            if result.overpaid.minor() > 0 { p { {crate::i18n::text("ชำระไว้เกิน ฿{0} · ยังไม่ใช่การอนุมัติคืนภาษี", &[money_label(result.overpaid).to_string()])} } }
                            dl { class: "tax-breakdown",
                                div { dt { {crate::i18n::text("เงินได้ก่อนหัก", &[])} } dd { "{money_label(result.gross)}" } }
                                div { dt { {crate::i18n::text("หักค่าใช้จ่าย", &[])} } dd { "{money_label(result.expenses)}" } }
                                for (label, amount) in result.allowances { if amount.minor() > 0 { div { dt { "{crate::i18n::tr(&label)}" } dd { "{money_label(amount)}" } } } }
                                div { dt { {crate::i18n::text("เงินบริจาคที่ใช้ได้", &[])} } dd { "{money_label(result.donation_allowed)}" } }
                                div { dt { {crate::i18n::text("เงินได้สุทธิ", &[])} } dd { "{money_label(result.net_taxable)}" } }
                                div { dt { {crate::i18n::text("วิธีที่ 1 อัตราก้าวหน้า", &[])} } dd { "{money_label(result.progressive)}" } }
                                div { dt { {crate::i18n::text("วิธีที่ 2 เงินได้ที่ไม่ใช่ ม.40(1) × 0.5%", &[])} } dd { "{money_label(result.minimum_raw)}" } }
                                div { dt { {crate::i18n::text("ภาษีก่อนหักเครดิต", &[])} } dd { "{money_label(result.tax_before_credits)}" } }
                                div { dt { {crate::i18n::text("ภาษีที่ชำระไว้", &[])} } dd { "{money_label(result.credits)}" } }
                            }
                            p { class: "field-hint", if result.minimum_applies { {crate::i18n::text("วิธีที่ 2 เกิน 5,000 บาท จึงเปรียบเทียบกับวิธีที่ 1 และใช้ยอดสูงกว่า", &[])} } else { {crate::i18n::text("วิธีที่ 2 ไม่เข้าเกณฑ์หรือไม่เกิน 5,000 บาท จึงได้รับยกเว้นเฉพาะวิธีที่ 2 ยังคงคำนวณวิธีที่ 1", &[])} } }
                            div { class: "tax-table-wrap", table { class: "tax-bands",
                                caption { {crate::i18n::text("ภาษีแยกตามขั้นเงินได้สุทธิ", &[])} }
                                thead { tr { th { {crate::i18n::text("อัตรา", &[])} } th { {crate::i18n::text("เงินได้ในขั้น", &[])} } th { {crate::i18n::text("ภาษี", &[])} } } }
                                tbody { for band in result.bands { tr { td { "{band.rate_percent}%" } td { "{money_label(band.taxable)}" } td { "{money_label(band.tax)}" } } } }
                            } }
                            p { class: "field-hint", {crate::i18n::text("ยอดชำระเพิ่มตัดเศษต่ำกว่าหนึ่งบาทหลังหักเครดิตแล้ว ภาษีเป็นศูนย์ไม่ได้แปลว่าไม่มีหน้าที่ยื่นแบบ", &[])} }
                            for warning in result.warnings { p { class: "tax-warning", "{warning}" } }
                        },
                    }
                } else { p { {crate::i18n::text("กรอกข้อมูลแล้วกดคำนวณ จะแสดงค่าใช้จ่าย ลดหย่อน ภาษีแต่ละขั้น และยอดชำระเพิ่มหรือชำระไว้เกิน", &[])} } }
                details { class: "tax-extra", summary { {crate::i18n::text("แหล่งกฎและขอบเขตการตรวจ", &[])} }
                    p { {crate::i18n::text("ตรวจแหล่งกฎ 24 กันยายน 2569 · {0}", &[TAX_RULE_VERSION.to_string()])} }
                    for (label, url) in TAX_SOURCES { p { a { href: url, target: "_blank", rel: "noopener noreferrer", "{crate::i18n::tr(&label)} ↗" } } }
                    p { a { href: "https://efiling.rd.go.th/rd-cms/bank", target: "_blank", rel: "noopener noreferrer", {crate::i18n::text("กรมสรรพากร: การตัดเศษสตางค์เมื่อชำระภาษี ↗", &[])} } }
                }
            }
        }
    }
}
