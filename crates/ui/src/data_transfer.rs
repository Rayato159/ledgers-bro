use crate::{
    components::{Icon, money_label},
    i18n::tr,
    state::UiState,
};
use dioxus::prelude::*;
use ledger_application::*;
use ledger_domain::EntryKind;

#[derive(Clone, PartialEq)]
enum Review {
    Template(CsvExport),
    Csv(Vec<PreparedEntry>),
    Backup(BackupReview),
}
#[derive(Clone, Copy)]
struct Transfer {
    store: UiState,
    review: Signal<Option<Review>>,
    error: Signal<Option<String>>,
    notice: Signal<Option<String>>,
    theme: crate::theme::Theme,
    locale: crate::i18n::Locale,
}
impl Transfer {
    fn run(mut self, command: Command, save: bool) {
        if *self.store.busy.peek() {
            return;
        }
        let gateway = self.store.gateway.peek().clone();
        self.store.busy.set(true);
        self.error.set(None);
        self.notice.set(None);
        spawn(async move {
            self.finish(gateway.0.request(command).await, save).await;
            self.store.busy.set(false);
        });
    }
    fn pick(mut self, kind: ImportFileKind) {
        if *self.store.busy.peek() {
            return;
        }
        let gateway = self.store.gateway.peek().clone();
        self.store.busy.set(true);
        self.error.set(None);
        self.notice.set(None);
        self.review.set(None);
        spawn(async move {
            match gateway.0.pick_document(kind).await {
                Ok(Some(bytes)) => {
                    let command = if kind == ImportFileKind::Csv {
                        Command::PreviewCsvImport(bytes)
                    } else {
                        Command::PreviewBackup(bytes)
                    };
                    self.finish(gateway.0.request(command).await, false).await;
                }
                Ok(None) => {}
                Err(e) => self.error.set(Some(tr(&e.to_string()))),
            }
            self.store.busy.set(false);
        });
    }
    async fn finish(mut self, result: Result<Response, AppError>, save: bool) {
        let gateway = self.store.gateway.peek().clone();
        let document = match result {
            Ok(Response::Csv(csv)) if save => Some(csv.into()),
            Ok(Response::Document(document)) => Some(document),
            Ok(Response::Csv(csv)) => {
                self.review.set(Some(Review::Template(csv)));
                None
            }
            Ok(Response::PreparedCsv(entries)) => {
                self.review.set(Some(Review::Csv(entries)));
                None
            }
            Ok(Response::BackupReview(review)) => {
                self.review.set(Some(Review::Backup(review)));
                None
            }
            Ok(Response::CommittedBatch(outcomes)) => {
                let added = outcomes
                    .iter()
                    .filter(|o| matches!(o, CommitOutcome::Saved(_)))
                    .count();
                let existing = outcomes.len() - added;
                self.notice.set(Some(format!(
                    "{}: {added} · {}: {existing}",
                    tr("บันทึกใหม่"),
                    tr("รายการเดิม ไม่บันทึกซ้ำ")
                )));
                self.review.set(None);
                self.reload().await;
                None
            }
            Ok(Response::BackupRestored(_)) => {
                self.review.set(None);
                self.notice
                    .set(Some(tr("กู้คืนข้อมูลสำเร็จ เลือกเมนูเพื่อดูข้อมูลได้เลย")));
                self.reload().await;
                if let Ok(Response::Preferences(p)) =
                    gateway.0.request(Command::LoadPreferences).await
                {
                    self.theme.0.set(p);
                    self.locale.0.set(if p.english {
                        crate::i18n::Language::English
                    } else {
                        crate::i18n::Language::Thai
                    });
                }
                None
            }
            Err(e) => {
                self.error.set(Some(tr(&e.to_string())));
                None
            }
            _ => {
                self.error.set(Some(tr("ดำเนินการไม่สำเร็จ กรุณาลองอีกครั้ง")));
                None
            }
        };
        if let Some(document) = document {
            match gateway.0.save_document(document).await {
                Ok(Some(name)) => self.notice.set(Some(format!("{}: {name}", tr("ส่งออกแล้ว")))),
                Ok(None) => {}
                Err(e) => self.error.set(Some(tr(&e.to_string()))),
            }
        }
    }
    async fn reload(mut self) {
        let gateway = self.store.gateway.peek().clone();
        match gateway.0.request(Command::Load).await {
            Ok(Response::Dashboard(view)) => self.store.view.set(Some(view)),
            _ => self.error.set(Some(tr(
                "บันทึกแล้ว แต่โหลดภาพรวมไม่สำเร็จ กรุณากดโหลดใหม่ก่อนทำรายการต่อ",
            ))),
        }
    }
}

#[component]
pub fn DataTransfer() -> Element {
    let mut transfer = Transfer {
        store: use_context::<UiState>(),
        review: use_signal(|| None),
        error: use_signal(|| None),
        notice: use_signal(|| None),
        theme: use_context(),
        locale: use_context(),
    };
    let busy = *transfer.store.busy.read();
    let view = transfer.store.view.read().clone();
    let empty = view.as_ref().is_some_and(|v| {
        v.accounts.is_empty()
            && v.entries.is_empty()
            && v.recurring.is_empty()
            && v.receivables.is_empty()
    });
    let currency = view.as_ref().map(|v| v.currency).unwrap_or_default();
    rsx! {
        section { class:"data-transfer", "aria-label":tr("นำเข้า ส่งออก และย้ายเครื่อง"),
        h2 {class:"preferences-section-title",{tr("นำเข้า ส่งออก และย้ายเครื่อง")}}
        div {class:"data-transfer-grid",
            section {class:"transfer-card",
                span {class:"transfer-icon",Icon {name:"notebook",size:27}}
                h3 {{tr("นำเข้ารายการ CSV")}}
                p {{tr("รับเฉพาะแม่แบบ Ledgers Bro แบบ UTF-8 สูงสุด 1,000 รายการ / 4 MB ตรวจทุกรายการก่อนยืนยัน")}}
                p {class:"field-hint",{tr("ชื่อบัญชีต้องตรงกับบัญชีที่มีอยู่ รายงานบัญชีคู่และ CSV ธนาคารใช้แทนแม่แบบนี้ไม่ได้")}}
                div {class:"transfer-actions",
                    button {class:"primary",disabled:busy,onclick:move |_|transfer.pick(ImportFileKind::Csv),Icon {name:"plus",size:18}{tr("เลือก CSV เพื่อนำเข้า")}}
                    button {class:"soft-button",disabled:busy,onclick:move |_|transfer.run(Command::CsvTemplate,false),Icon {name:"file",size:18}{tr("เปิดไฟล์ตัวอย่าง")}}
                }
            }
            section {class:"transfer-card",
                span {class:"transfer-icon",Icon {name:"download",size:27}}
                h3 {{tr("รายงานสำหรับ Excel")}}
                p {{tr("ส่งออกบัญชีคู่ สมุดรายวัน และรายงานยอดเป็น CSV เลือกภาษาไทยหรืออังกฤษได้")}}
                div {class:"transfer-actions",button {class:"soft-button",disabled:busy,onclick:move |_|transfer.store.export_form.set(true),Icon {name:"download",size:18}{tr("ส่งออก CSV สำหรับ Excel")}}}
            }
            section {class:"transfer-card",
                span {class:"transfer-icon",Icon {name:"wallet",size:27}}
                h3 {{tr("สำรองข้อมูลผู้ใช้นี้")}}
                p {{tr("เก็บบัญชี รายการ หนี้ บิล ข้อมูลภาษีในรายการ พอร์ตเหรียญ และการตั้งค่าที่บันทึกแล้วในไฟล์ .lbro สำหรับย้ายเครื่อง")}}
                p {class:"field-hint",{tr("ไม่รวมรหัสผ่าน โมเดล AI หรือแบบร่างที่ยังไม่บันทึก ไฟล์มีข้อมูลการเงินของคุณ เก็บไว้ในที่ส่วนตัว")}}
                div {class:"transfer-actions",button {class:"primary",disabled:busy,onclick:move |_|transfer.run(Command::ExportBackup,true),Icon {name:"download",size:18}{tr("ส่งออกไฟล์สำรอง .lbro")}}}
            }
            section {class:"transfer-card",
                span {class:"transfer-icon",Icon {name:"arrow-right",size:27}}
                h3 {{tr("ย้ายข้อมูลเข้าผู้ใช้นี้")}}
                p {{tr("บนเครื่องใหม่ สร้างผู้ใช้และตั้งรหัสผ่านก่อน แล้วเลือกไฟล์ .lbro เพื่อตรวจและกู้คืน")}}
                if !empty {p {class:"field-hint",{tr("ผู้ใช้นี้มีข้อมูลแล้ว สร้างผู้ใช้ใหม่เพื่อกู้คืน ข้อมูลเดิมจะไม่ถูกเขียนทับ")}}}
                div {class:"transfer-actions",button {class:"soft-button",disabled:busy||!empty,onclick:move |_|transfer.pick(ImportFileKind::Backup),Icon {name:"file",size:18}{tr("เลือกไฟล์สำรองเพื่อกู้คืน")}}}
            }
        }
        if (transfer.review)().is_none() {
            if busy {p {class:"transfer-pending",role:"status",Icon {name:"refresh",size:20}{tr("กำลังดำเนินการ…")}}}
            TransferFeedback { error:transfer.error, notice:transfer.notice }
        }
        if let Some(review)=(transfer.review)() {
            dialog {id:"transfer-review",class:"account-dialog transfer-dialog","aria-labelledby":"transfer-review-title",onmounted:move |_|{let _=document::eval("document.getElementById('transfer-review').showModal()");},oncancel:move |e|{e.prevent_default();if !*transfer.store.busy.peek(){transfer.review.set(None);}},
                div {class:"section-heading",h2 {id:"transfer-review-title",{tr(match &review {Review::Template(_)=>"ไฟล์ตัวอย่าง CSV",Review::Csv(_)=>"ตรวจรายการก่อนนำเข้า",Review::Backup(_)=>"ตรวจข้อมูลก่อนกู้คืน"})}}
                    button {class:"icon-button","aria-label":tr("ปิด"),disabled:busy,onclick:move |_|transfer.review.set(None),Icon {name:"close",size:20}}
                }
                match review {
                    Review::Template(csv)=>rsx! {
                        p {{tr("แก้ชื่อบัญชีให้ตรงกับของคุณ ใช้ external_id ที่ไม่ซ้ำกันต่อรายการ ใช้วันที่ ค.ศ. แบบ YYYY-MM-DD และจำนวนเงินทศนิยมไม่เกิน 2 ตำแหน่ง")}}
                        pre {class:"csv-example", "{csv.contents}"}
                        p {class:"field-hint", "kind: expense / income / transfer"}
                        p {class:"field-hint", "category: food, snacks, rent, luxury, supplies, medical, other_expense, salary, freelance, interest, other_income"}
                        p {class:"field-hint",{tr("โอนเงินให้ระบุ destination และเว้น category ว่าง รายรับ/รายจ่ายให้เว้น destination ว่าง")}}
                        button {class:"primary",disabled:busy,onclick:move |_|transfer.run(Command::CsvTemplate,true),Icon {name:"download",size:18}{tr("บันทึกไฟล์ตัวอย่าง")}}
                    },
                    Review::Csv(entries)=>{
                        let count=entries.len();let confirm=entries.clone();
                        rsx! {
                            p {"{count} " {tr("รายการ · ยังไม่บันทึก")}}
                            div {class:"import-rows",
                                for (index,prepared) in entries.iter().enumerate() {
                                    {let (kind,amount,account,destination,category)=match prepared.entry.kind(){
                                        EntryKind::Expense {account,amount,category}=>("รายจ่าย",amount.money(),*account,None,Some(*category)),
                                        EntryKind::Income {account,amount,category}=>("รายรับ",amount.money(),*account,None,Some(*category)),
                                        EntryKind::Transfer {from,to,amount}=>("โอนเงิน",amount.money(),*from,Some(*to),None),
                                        _=>return rsx!{},
                                    };
                                    let name=|id|view.as_ref().and_then(|v|v.accounts.iter().find(|a|a.account.id()==id)).map(|a|a.account.name().as_str().to_owned()).unwrap_or_default();
                                    rsx! {article {class:"import-row",
                                        div {strong {"{index+1}. {tr(kind)}"} span {"{currency.code()} {money_label(amount)}"}}
                                        p {"{prepared.entry.date()} · {name(account)}" if let Some(to)=destination {" → {name(to)}"} if let Some(category)=category {" · {tr(category.label())}"}}
                                        p {class:"entry-note-body","{prepared.entry.note().as_str()}"}
                                    }} }
                                }
                            }
                            p {class:"field-hint",{tr("หาก external_id และข้อมูลตรงกับรายการที่เคยนำเข้า จะไม่นับซ้ำ ถ้าข้อมูลต่างกันระบบจะหยุดให้ตรวจใหม่")}}
                            button {class:"primary full-width",disabled:busy,onclick:move |_|transfer.run(Command::CommitCsvImport(confirm.clone()),false),{tr("ยืนยันนำเข้าทั้งชุด")}}
                        }
                    },
                    Review::Backup(review)=>{
                        let s=review.summary.clone();
                        rsx! {
                            dl {class:"backup-summary",
                                dt {{tr("บัญชี")}} dd {"{s.accounts}"}
                                dt {{tr("รายการ")}} dd {"{s.transactions}"}
                                dt {{tr("หนี้และบิล")}} dd {"{s.plans}"}
                                dt {{tr("ลูกหนี้")}} dd {"{s.receivables}"}
                                dt {{tr("สกุลเงินสมุดบัญชี")}} dd {"{s.currency.code()}"}
                            }
                            p {{tr("กู้คืนข้อมูลเข้าโปรไฟล์ปัจจุบัน ไม่เปลี่ยนชื่อผู้ใช้หรือรหัสผ่าน")}}
                            button {class:"primary full-width",disabled:busy||!empty,onclick:move |_|transfer.run(Command::RestoreBackup(review.clone()),false),{tr("ยืนยันกู้คืนข้อมูล")}}
                        }
                    },
                }
                TransferFeedback { error:transfer.error, notice:transfer.notice }
            }
        }
        }
    }
}

#[component]
fn TransferFeedback(
    mut error: Signal<Option<String>>,
    mut notice: Signal<Option<String>>,
) -> Element {
    rsx! {
        if let Some(message)=error() {
            div {class:"transfer-feedback transfer-feedback-error",role:"alert",
                span {class:"transfer-feedback-icon",Icon {name:"close",size:20}}
                p {"{message}"}
                button {class:"transfer-dismiss",r#type:"button","aria-label":tr("ปิดข้อความ"),onclick:move |_|error.set(None),Icon {name:"close",size:18}}
            }
        }
        if let Some(message)=notice() {
            div {class:"transfer-feedback transfer-feedback-success",role:"status",
                span {class:"transfer-feedback-icon",Icon {name:"check",size:20}}
                p {"{message}"}
                button {class:"transfer-dismiss",r#type:"button","aria-label":tr("ปิดข้อความ"),onclick:move |_|notice.set(None),Icon {name:"close",size:18}}
            }
        }
    }
}
