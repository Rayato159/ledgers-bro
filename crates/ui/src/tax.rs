use crate::components::money_label;
use dioxus::prelude::*;
use ledger_application::{TAX_RULE_VERSION, TAX_SOURCES, TaxWorksheet, calculate_tax};

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
            label { r#for: "tax-{group}-{index}", "{label}" }
            input { id: "tax-{group}-{index}", inputmode: "decimal", maxlength: 18, required: true, value,
                oninput: move |event| { let mut state = session.write(); *amount_field(&mut state.worksheet, group, index) = event.value(); state.submitted = false; }
            }
        }
    }
}

#[component]
pub(crate) fn TaxPage() -> Element {
    let mut session = use_context::<Signal<TaxSession>>();
    let state = session.read().clone();
    let result = state.submitted.then(|| calculate_tax(&state.worksheet));
    rsx! {
        section { class: "page-heading", div { h1 { "คำนวณภาษีบุคคลธรรมดา" } p { class: "muted", "คำนวณรายปีจากข้อมูลและสิทธิที่ยืนยัน · ยังไม่ได้ยื่นแบบ" } } }
        div { class: "tax-calculator",
            form { class: "card tax-inputs", onsubmit: move |event| {
                event.prevent_default(); session.write().submitted = true;
                let _ = document::eval("requestAnimationFrame(() => { const result = document.getElementById('tax-result'); result.focus({preventScroll: true}); result.scrollIntoView({block: 'start'}); })");
            },
                label { r#for: "tax-year", "ปีภาษี (ปีที่ได้รับเงิน)" }
                select { id: "tax-year", value: state.worksheet.year.to_string(), onchange: move |event| {
                    if let Ok(year) = event.value().parse() { let mut state = session.write(); state.worksheet.year = year; state.worksheet.eligibility_confirmed = false; state.submitted = false; }
                }, option { value: "2569", "2569 · ประมาณการปีปัจจุบัน" } option { value: "2568", "2568" } }
                p { class: "field-hint", "ทุกช่องเป็นยอดรวมทั้งปี หน่วยบาท กรอก 0 ถ้าไม่มี ข้อมูลค้างไว้ระหว่างเปลี่ยนหน้า แต่ยังไม่บันทึกแบบภาษีเมื่อปิดแอป" }
                h2 { "1. เงินได้ก่อนหักภาษี" }
                p { "ใช้หนังสือ 50 ทวิและหลักฐานรายได้ ไม่ใช้ยอดสุทธิที่โอนเข้าธนาคารแทน และไม่รวม VAT เป็นรายได้" }
                TaxAmount { label: "ม.40(1) เงินเดือน ค่าจ้าง โบนัส", group: 0, index: 0 }
                TaxAmount { label: "ม.40(2) ค่ารับทำงาน ค่าธรรมเนียม ค่านายหน้า", group: 0, index: 1 }
                p { class: "field-hint", "สองประเภทรวมกันหักค่าใช้จ่าย 50% ไม่เกิน 100,000 บาท ค่าจ้างงานบางแบบเป็น ม.40(8) ต้องตรวจตามข้อเท็จจริง" }
                details { class: "tax-extra", summary { "เงินได้ประเภทอื่น ม.40(3)–(8)" }
                    for (index, label) in [(2, "ม.40(3) ค่าสิทธิที่หักค่าใช้จ่ายจริงได้"), (3, "ม.40(4) ดอกเบี้ยที่เลือกนำมารวมคำนวณ"), (4, "ม.40(5) ค่าเช่าทรัพย์สิน"), (5, "ม.40(6) วิชาชีพอิสระ"), (6, "ม.40(7) รับเหมาพร้อมวัสดุสำคัญ"), (7, "ม.40(8) ธุรกิจและเงินได้อื่น")] {
                        TaxAmount { label: label.to_owned(), group: 0, index }
                        if index != 3 { TaxAmount { label: format!("ค่าใช้จ่ายจริงที่มีสิทธิหัก ม.40({})", index + 1), group: 1, index } }
                    }
                    p { class: "field-hint", "ค่าใช้จ่ายจริงต้องจำเป็น สมควร เกี่ยวกับเงินได้นั้น และมีหลักฐาน รุ่นนี้ยังไม่เปรียบเทียบวิธีเหมาตามประเภทย่อยให้ ถ้าต้องการวิธีเหมาให้เลือกกรณีที่ยังไม่รองรับด้านล่าง" }
                }
                h2 { "2. ค่าลดหย่อนที่มีสิทธิ" }
                p { "ส่วนตัว 60,000 บาท คำนวณให้อัตโนมัติ" }
                label { class: "tax-check", input { r#type: "checkbox", checked: state.worksheet.spouse_no_income, onchange: move |event| { let mut state = session.write(); state.worksheet.spouse_no_income = event.checked(); state.submitted = false; } } "คู่สมรสจดทะเบียนไม่มีเงินได้และเข้าเงื่อนไขสิทธิ 60,000 บาท" }
                details { class: "tax-extra", summary { "บุตร บิดามารดา และผู้พิการ" }
                    for (index, label, value) in [(0, "บุตรที่มีสิทธิ (คน)", state.worksheet.eligible_children), (1, "ในจำนวนข้างต้น: บุตรชอบด้วยกฎหมายคนที่ 2 เป็นต้นไป เกิดตั้งแต่ 2561 (คน)", state.worksheet.additional_children), (2, "บิดามารดาที่มีสิทธิ ไม่ใช้ซ้ำกับผู้อื่น (คน)", state.worksheet.eligible_parents), (3, "ผู้พิการ/ทุพพลภาพที่มีสิทธิอุปการะ (คน)", state.worksheet.eligible_disabled)] {
                        label { r#for: "tax-count-{index}", "{label}" }
                        input { id: "tax-count-{index}", r#type: "number", min: 0, max: 100, step: 1, value: value.to_string(), required: true, onchange: move |event| { if let Ok(value) = event.value().parse() { let mut state = session.write(); match index { 0 => state.worksheet.eligible_children = value, 1 => state.worksheet.additional_children = value, 2 => state.worksheet.eligible_parents = value, _ => state.worksheet.eligible_disabled = value }; state.submitted = false; } } }
                    }
                    p { class: "field-hint", "บุตรต้องเข้าเงื่อนไขอายุ/การศึกษา/เงินได้; บุตรบุญธรรมมีเพดานจำนวนและไม่ได้สิทธิเพิ่มเติม บิดามารดาต้องอายุ 60 ปีขึ้นไป เงินได้ไม่เกิน 30,000 บาท และมี ลย.03 ผู้พิการต้องมีหลักฐานและสิทธิอุปการะตามเกณฑ์กรมสรรพากร" }
                }
                for (index, label) in [(0, "เบี้ยประกันชีวิตตนเองที่เข้าเงื่อนไข"), (1, "เบี้ยประกันสุขภาพตนเองที่เข้าเงื่อนไข"), (2, "ประกันสุขภาพบิดามารดา เฉพาะส่วนสิทธิของตน"), (3, "ดอกเบี้ยบ้าน เฉพาะส่วนสิทธิของตน"), (4, "ประกันสังคม ม.33 ของตนเอง ตามจ่ายจริง"), (5, "บริจาคทั่วไปที่มีสิทธิหัก 1 เท่า")] { TaxAmount { label: label.to_owned(), group: 2, index } }
                p { class: "field-hint", "ประกันชีวิตต้องเข้าเงื่อนไขสัญญา 10 ปีขึ้นไปและแจ้งใช้สิทธิ ประกันสุขภาพไม่เกิน 25,000 บาท รวมประกันชีวิตไม่เกิน 100,000 บาท บริจาคทั่วไปไม่เกิน 10% หลังลดหย่อน; กรณี e-Donation ให้ตรวจหลักฐานของปีที่ใช้สิทธิ" }
                h2 { "3. ภาษีที่ชำระไว้แล้ว" }
                TaxAmount { label: "ภาษีหัก ณ ที่จ่ายของเงินได้ที่นำมารวมครั้งนี้", group: 2, index: 6 }
                TaxAmount { label: "ภาษีครึ่งปี/ชำระล่วงหน้าที่นำมาเครดิตได้", group: 2, index: 7 }
                details { class: "tax-extra", open: true, summary { "ตรวจขอบเขตก่อนคำนวณ" }
                    p { "รองรับการคำนวณรายปีของบุคคลธรรมดาผู้อยู่ในไทย ยื่นแยกของตนเอง ไม่ใช่ VAT หรือภาษีธุรกิจทุกประเภท" }
                    label { class: "tax-check", input { r#type: "checkbox", checked: state.worksheet.unsupported_items, onchange: move |event| { let mut state = session.write(); state.worksheet.unsupported_items = event.checked(); state.submitted = false; } }
                        "มีกรณีที่ยังไม่รองรับ: RMF/กองทุนเกษียณ/ประกันบำนาญ/Thai ESG, สิทธิพิเศษรายปี, ประกันสังคม ม.39/40, ประกันชีวิตคู่สมรส, เงินบริจาค 2 เท่า, ยกเว้นผู้สูงอายุ/ผู้พิการ, เงินได้ต่างประเทศ/เครดิตภาษีต่างประเทศ, เงินปันผลพร้อมเครดิต, เงินออกจากงานแยกคำนวณ, วิธีเหมา ม.40(3)–(8), ภาษีอัตราพิเศษ หรือยื่นรวมคู่สมรส"
                    }
                    label { class: "tax-check", input { r#type: "checkbox", checked: state.worksheet.eligibility_confirmed, onchange: move |event| { let mut state = session.write(); state.worksheet.eligibility_confirmed = event.checked(); state.submitted = false; } } "ตรวจแล้วว่าข้อมูลเงินได้ครบ ค่าใช้จ่ายมีหลักฐาน และจำนวนสิทธิลดหย่อนที่กรอกเข้าเงื่อนไขของปีนี้ ไม่ใช้สิทธิซ้ำ" }
                }
                button { class: "primary full-width", r#type: "submit", "คำนวณภาษีตามข้อมูลที่กรอก" }
            }
            section { class: "card tax-result", id: "tax-result", tabindex: "-1", "aria-live": "polite",
                h2 { "ผลคำนวณและที่มา" }
                if let Some(result) = result {
                    match result {
                        Err(error) => rsx! { p { class: "form-error", role: "alert", "{error}" } },
                        Ok(result) => rsx! {
                            span { class: "status-pill", "ปี {result.year} · ผลตามข้อมูลที่ยืนยัน" }
                            div { class: "tax-total", small { "ยอดต้องชำระเพิ่ม" } strong { "฿{money_label(result.payable)}" } }
                            if result.overpaid.minor() > 0 { p { "ชำระไว้เกิน ฿{money_label(result.overpaid)} · ยังไม่ใช่การอนุมัติคืนภาษี" } }
                            dl { class: "tax-breakdown",
                                div { dt { "เงินได้ก่อนหัก" } dd { "{money_label(result.gross)}" } }
                                div { dt { "หักค่าใช้จ่าย" } dd { "{money_label(result.expenses)}" } }
                                for (label, amount) in result.allowances { if amount.minor() > 0 { div { dt { "{label}" } dd { "{money_label(amount)}" } } } }
                                div { dt { "เงินบริจาคที่ใช้ได้" } dd { "{money_label(result.donation_allowed)}" } }
                                div { dt { "เงินได้สุทธิ" } dd { "{money_label(result.net_taxable)}" } }
                                div { dt { "วิธีที่ 1 อัตราก้าวหน้า" } dd { "{money_label(result.progressive)}" } }
                                div { dt { "วิธีที่ 2 เงินได้ที่ไม่ใช่ ม.40(1) × 0.5%" } dd { "{money_label(result.minimum_raw)}" } }
                                div { dt { "ภาษีก่อนหักเครดิต" } dd { "{money_label(result.tax_before_credits)}" } }
                                div { dt { "ภาษีที่ชำระไว้" } dd { "{money_label(result.credits)}" } }
                            }
                            p { class: "field-hint", if result.minimum_applies { "วิธีที่ 2 เกิน 5,000 บาท จึงเปรียบเทียบกับวิธีที่ 1 และใช้ยอดสูงกว่า" } else { "วิธีที่ 2 ไม่เข้าเกณฑ์หรือไม่เกิน 5,000 บาท จึงได้รับยกเว้นเฉพาะวิธีที่ 2 ยังคงคำนวณวิธีที่ 1" } }
                            div { class: "tax-table-wrap", table { class: "tax-bands",
                                caption { "ภาษีแยกตามขั้นเงินได้สุทธิ" }
                                thead { tr { th { "อัตรา" } th { "เงินได้ในขั้น" } th { "ภาษี" } } }
                                tbody { for band in result.bands { tr { td { "{band.rate_percent}%" } td { "{money_label(band.taxable)}" } td { "{money_label(band.tax)}" } } } }
                            } }
                            p { class: "field-hint", "ยอดชำระเพิ่มตัดเศษต่ำกว่าหนึ่งบาทหลังหักเครดิตแล้ว ภาษีเป็นศูนย์ไม่ได้แปลว่าไม่มีหน้าที่ยื่นแบบ" }
                            for warning in result.warnings { p { class: "tax-warning", "{warning}" } }
                        },
                    }
                } else { p { "กรอกข้อมูลแล้วกดคำนวณ จะแสดงค่าใช้จ่าย ลดหย่อน ภาษีแต่ละขั้น และยอดชำระเพิ่มหรือชำระไว้เกิน" } }
                details { class: "tax-extra", summary { "แหล่งกฎและขอบเขตการตรวจ" }
                    p { "ตรวจแหล่งกฎ 24 กันยายน 2569 · {TAX_RULE_VERSION}" }
                    for (label, url) in TAX_SOURCES { p { a { href: url, target: "_blank", rel: "noopener noreferrer", "{label} ↗" } } }
                    p { a { href: "https://efiling.rd.go.th/rd-cms/bank", target: "_blank", rel: "noopener noreferrer", "กรมสรรพากร: การตัดเศษสตางค์เมื่อชำระภาษี ↗" } }
                }
            }
        }
    }
}
