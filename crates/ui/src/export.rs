use crate::{components::*, state::UiState};
use dioxus::prelude::*;
use ledger_application::{AccountingReport, Command, ExportLanguage, ExportOptions};

#[component]
pub fn ExportDialog() -> Element {
    let mut store = use_context::<UiState>();
    let mut report = use_signal(|| AccountingReport::Journal);
    let mut language = use_signal(|| ExportLanguage::Thai);
    rsx! {
        dialog { id: "export-dialog", class: "account-dialog", "aria-labelledby": "export-title",
            onmounted: move |_| { let _ = document::eval("document.getElementById('export-dialog').showModal()"); },
            oncancel: move |event| { event.prevent_default(); if !*store.busy.read() { store.export_form.set(false); } },
            form { onsubmit: move |event| { event.prevent_default(); store.send(Command::ExportCsv(ExportOptions { report: report(), language: language() })); },
                div { class: "section-heading", h2 { id: "export-title", {crate::i18n::text("ส่งออกรายงานบัญชีคู่", &[])} }
                    button { r#type: "button", class: "icon-button", "aria-label": crate::i18n::text("ปิดหน้าต่างส่งออก", &[]), disabled: *store.busy.read(), onclick: move |_| store.export_form.set(false), Icon { name: "close", size: 20 } }
                }
                label { r#for: "export-report", {crate::i18n::text("รายงาน", &[])} }
                select { id: "export-report", value: report().code(), disabled: *store.busy.read(),
                    onchange: move |event| { if let Some(value) = AccountingReport::ALL.into_iter().find(|item| item.code() == event.value()) { report.set(value); } },
                    for item in AccountingReport::ALL { option { value: item.code(), {item.label(if crate::i18n::english() { ExportLanguage::English } else { ExportLanguage::Thai })} } }
                }
                label { r#for: "export-language", {crate::i18n::text("ภาษาในไฟล์", &[])} }
                select { id: "export-language", value: if language() == ExportLanguage::Thai { "th" } else { "en" }, disabled: *store.busy.read(),
                    onchange: move |event| language.set(if event.value() == "en" { ExportLanguage::English } else { ExportLanguage::Thai }),
                    option { value: "th", {crate::i18n::text("ไทย", &[])} } option { value: "en", "English" }
                }
                p { class: "field-hint", {crate::i18n::text("รวมข้อมูลตั้งแต่เริ่มบันทึกถึงวันนี้ แยกเดบิต–เครดิต รวมยอดเริ่มต้นและรายการกลับบัญชี วันที่ใช้ ค.ศ. ชื่อบัญชีและรายละเอียดคงข้อความที่กรอกไว้", &[])} }
                p { class: "field-hint", {crate::i18n::text("รายงานนี้อิงยอดที่บันทึก ยังไม่แยก VAT หรือภาษีหัก ณ ที่จ่าย และไม่ใช่แบบยื่นภาษี", &[])} }
                if let Some((true, message)) = store.notice.read().clone() { p { class: "form-error", role: "alert", "{message}" } }
                button { class: "primary full-width", r#type: "submit", disabled: *store.busy.read(), Icon { name: "download", size: 20 }
                    if *store.busy.read() { {crate::i18n::text("กำลังส่งออก…", &[])} } else { {crate::i18n::text("เลือกที่บันทึกไฟล์ CSV", &[])} }
                }
            }
        }
    }
}
