use crate::Gateway;
use crate::receipt::{ReceiptAttachment, ReceiptDestination, ReceiptReview};
use dioxus::prelude::*;
use ledger_application::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Overview,
    Accounts,
    Transactions,
    Chat,
    Manual,
    Tax,
    Recurring,
    Receivables,
    Settings,
}
impl Page {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Settings => "ตั้งค่า",
            Self::Overview => "ภาพรวม",
            Self::Accounts => "บัญชี",
            Self::Transactions => "รายการ",
            Self::Chat => "เพิ่มรายการ",
            Self::Manual => "กรอกเอง",
            Self::Tax => "ภาษี",
            Self::Recurring => "หนี้และบิล",
            Self::Receivables => "ลูกหนี้",
        }
    }
    pub const fn icon(self) -> &'static str {
        match self {
            Self::Settings => "settings",
            Self::Overview => "home",
            Self::Accounts => "wallet",
            Self::Transactions => "notebook",
            Self::Chat => "chat",
            Self::Manual => "edit",
            Self::Tax => "file",
            Self::Recurring => "calendar",
            Self::Receivables => "wallet",
        }
    }
}

#[derive(Clone, Copy)]
pub struct UiState {
    pub gateway: Signal<Gateway>,
    pub view: Signal<Option<Dashboard>>,
    pub page: Signal<Page>,
    pub busy: Signal<bool>,
    pub notice: Signal<Option<(bool, String)>>,
    pub account_form: Signal<bool>,
    pub export_form: Signal<bool>,
    pub account_deletion: Signal<Option<AccountDeletion>>,
    pub input: Signal<Option<EntryInput>>,
    pub prepared: Signal<Option<PreparedEntry>>,
    pub guidance: Signal<String>,
    pub composer: Signal<String>,
    pub receipts: Signal<Vec<ReceiptAttachment>>,
    pub scan_progress: Signal<String>,
    pub scan_cancel: Signal<Option<Arc<AtomicBool>>>,
    pub model_operation: Signal<Option<ModelOperation>>,
    pub model_choices: Signal<Option<(String, Vec<ModelDraft>)>>,
    pub prompt_drafts: Signal<Option<(String, Vec<PromptDraft>)>>,
    pub prompt_prepared: Signal<Option<PromptPlan>>,
    pub batch: Signal<Option<(String, Vec<ModelDraft>)>>,
    pub batch_prepared: Signal<Option<Vec<PreparedEntry>>>,
    pub recurring_prepared: Signal<Option<PreparedRecurringPayment>>,
    pub receivable_review: Signal<Option<ReceivableReview>>,
    pub repayment_form: Signal<bool>,
    pub repayment_selection: Signal<Option<ledger_domain::ReceivableId>>,
}
impl UiState {
    pub fn cancel_model(self) {
        if let Some(operation) = self.model_operation.peek().as_ref() {
            operation.cancelled.store(true, Ordering::Relaxed);
        }
    }
    pub fn resolve_text(mut self, text: String) {
        if *self.busy.peek() {
            return;
        }
        let operation = ModelOperation::default();
        self.model_operation.set(Some(operation.clone()));
        self.model_choices.set(None);
        self.prompt_drafts.set(None);
        self.prompt_prepared.set(None);
        self.batch.set(None);
        self.batch_prepared.set(None);
        self.input.set(None);
        self.prepared.set(None);

        self.guidance.set("กำลังอ่านรายการในเครื่อง…".into());
        self.notice.set(None);
        self.busy.set(true);
        let gateway = self.gateway.peek().clone();
        dioxus::dioxus_core::spawn_forever(async move {
            let result = await_model(
                gateway.0.resolve_text(text, operation.clone()),
                &operation,
                std::time::Duration::from_secs(90),
            )
            .await;
            match result {
                Ok(None) => self.guidance.set("ยกเลิกแล้ว ยังไม่มีรายการถูกบันทึก".into()),
                result => {
                    self.guidance.set(String::new());
                    match result {
                        Ok(Some(Response::Resolved(resolution))) => {
                            self.apply_resolution(resolution)
                        }
                        Err(error) => self.notice.set(Some((true, error.to_string()))),
                        _ => self.notice.set(Some((true, "อ่านรายการไม่สำเร็จ".into()))),
                    }
                }
            }
            self.model_operation.set(None);
            self.busy.set(false);
        });
    }
    fn apply_resolution(mut self, resolution: QuickResolution) {
        self.prepared.set(None);
        self.model_choices.set(None);
        match resolution {
            QuickResolution::Actions { source, drafts } => {
                self.input.set(None);
                self.batch.set(None);
                self.prompt_prepared.set(None);
                self.prompt_drafts.set(Some((source, drafts)));
                self.guidance
                    .set("ตรวจทุกคำสั่งและเติมข้อมูลที่ขาดก่อนยืนยัน ยังไม่มีข้อมูลถูกบันทึก".into());
            }
            QuickResolution::Batch { source, drafts } => {
                self.input.set(None);
                self.batch_prepared.set(None);
                self.batch.set(Some((source, drafts)));
                self.guidance.set(
                    "แยกรายการแล้ว ตรวจข้อความต้นฉบับและเติมช่องที่ขาดด้านล่าง ยังไม่มีรายการถูกบันทึก".into(),
                );
            }
            QuickResolution::Draft { input, guidance } => {
                self.input.set(Some(input));
                self.guidance.set(guidance);
            }
            QuickResolution::Choices { source, drafts } => {
                self.input.set(None);
                self.model_choices.set(Some((source, drafts)));
            }
            QuickResolution::Summary => self.page.set(Page::Overview),
            QuickResolution::Help => self
                .guidance
                .set("ใช้ตัวอย่างด้านบน หรือเลือกแบบฟอร์ม กรอกชื่อที่มีช่องว่างในเครื่องหมายคำพูด".into()),
        }
    }
    pub fn scan_receipts(
        self,
        files: Vec<dioxus::html::FileData>,
        destination: ReceiptDestination,
    ) {
        if !files.is_empty() {
            self.run_receipt_scan(Some(files), destination);
        }
    }
    pub fn pick_receipts(self, destination: ReceiptDestination) {
        self.run_receipt_scan(None, destination);
    }
    fn run_receipt_scan(
        mut self,
        files: Option<Vec<dioxus::html::FileData>>,
        destination: ReceiptDestination,
    ) {
        if *self.busy.peek() {
            return;
        }
        if self
            .view
            .peek()
            .as_ref()
            .is_some_and(|v| v.currency != ledger_domain::Currency::Thb)
        {
            self.notice.set(Some((true, "Receipt OCR currently supports THB ledgers only. Use manual entry in your ledger currency.".into())));
            return;
        }
        let existing = self.receipts.peek().len();
        let existing_bytes = self
            .receipts
            .peek()
            .iter()
            .filter_map(|a| a.review.as_ref().ok())
            .map(|r| r.byte_len)
            .sum::<u64>();
        if existing >= MAX_RECEIPT_IMAGES
            || files.as_ref().is_some_and(|files| {
                files.len() + existing > MAX_RECEIPT_IMAGES
                    || files.iter().fold(existing_bytes, |total, file| {
                        total.saturating_add(file.size())
                    }) > MAX_RECEIPT_BATCH_BYTES
            })
        {
            self.notice.set(Some((
                true,
                "เพิ่มได้ไม่เกิน 8 รูปต่อชุด รวมไม่เกิน 128 MB กรุณาแบ่งบันทึกเป็นชุดเล็กลง".into(),
            )));
            return;
        }
        let Some(today) = self.view.peek().as_ref().map(|v| v.today) else {
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        self.scan_cancel.set(Some(cancel.clone()));
        self.busy.set(true);
        self.notice.set(None);
        self.scan_progress
            .set("กำลังเลือกรูปและอ่านใบเสร็จในเครื่อง…".into());
        let gateway = self.gateway.peek().clone();
        dioxus::dioxus_core::spawn_forever(async move {
            let result = if let Some(files) = files {
                let mut results = Vec::new();
                let count = files.len();
                for (index, file) in files.into_iter().enumerate() {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    self.scan_progress
                        .set(format!("กำลังอ่านใบเสร็จ {} / {}…", index + 1, count));
                    let name = file.name();
                    let result = if file.size() > MAX_RECEIPT_BYTES as u64 {
                        Err(AppError::Input("รูปใบเสร็จต้องไม่เกิน 32 MB".into()))
                    } else {
                        await_receipt(gateway.0.scan_receipt(file, cancel.clone()), &cancel, 90)
                            .await
                    };
                    results.push((name, result));
                }
                Ok(results)
            } else {
                await_receipt(gateway.0.pick_receipts(cancel.clone()), &cancel, 840).await
            };
            if cancel.load(Ordering::Relaxed) {
                self.notice.set(Some((
                    false,
                    "หยุดอ่านใบเสร็จแล้ว (ยกเลิกหรือหมดเวลา) ข้อความและรายการเดิมยังอยู่".into(),
                )));
            } else {
                match result {
                    Ok(results)
                        if results.len() + existing > MAX_RECEIPT_IMAGES
                            || results
                                .iter()
                                .filter_map(|(_, result)| result.as_ref().ok())
                                .fold(existing_bytes, |total, (image, _)| {
                                    total.saturating_add(image.bytes().len() as u64)
                                })
                                > MAX_RECEIPT_BATCH_BYTES =>
                    {
                        self.notice.set(Some((
                            true,
                            "เพิ่มได้ไม่เกิน 8 รูปต่อชุด รวมไม่เกิน 128 MB แบ่งเลือกรูปให้น้อยลง".into(),
                        )))
                    }
                    Ok(results) => {
                        let mut drafts = self
                            .batch
                            .peek()
                            .as_ref()
                            .map(|(_, d)| d.clone())
                            .unwrap_or_default();
                        if destination == ReceiptDestination::Manual
                            && drafts.is_empty()
                            && let Some(input) = self.input.peek().clone()
                            && (!input.amount.is_empty()
                                || !input.note.is_empty()
                                || input.account.is_some())
                        {
                            drafts.push(ModelDraft {
                                input,
                                guidance: "รายการที่กรอกไว้ก่อนเพิ่มใบเสร็จ".into(),
                            });
                        }
                        if destination == ReceiptDestination::Manual
                            && drafts.len() + results.len() > MAX_BATCH_ENTRIES
                        {
                            self.notice.set(Some((
                                true,
                                "รวมรายการเดิมแล้วเกิน 8 รายการ บันทึกชุดเดิมก่อนเพิ่มใบเสร็จ".into(),
                            )));
                        } else {
                            let mut blocks = Vec::new();
                            let mut attachments = self.receipts.peek().clone();
                            let mut successes = 0;
                            for (name, result) in results {
                                let number = attachments.len() + 1;
                                let review = result.and_then(|(image, text)| {
                                    analyze_receipt(&text, today)
                                        .map(|analysis| ReceiptReview::new(image, analysis))
                                });
                                if let Ok(review) = &review {
                                    successes += 1;
                                    blocks.push(format_receipt_prompt(
                                        &review.analysis,
                                        today,
                                        number,
                                    ));
                                    drafts.push(ModelDraft {
                                        input: review.analysis.draft(today),
                                        guidance: format!(
                                            "ใบเสร็จ {number}: ตรวจยอด วันที่ และเลือกบัญชีกับหมวดก่อนบันทึก"
                                        ),
                                    });
                                }
                                attachments.push(ReceiptAttachment {
                                    name,
                                    review: review.map_err(|e| e.to_string()),
                                });
                            }
                            self.receipts.set(attachments);
                            if successes > 0 {
                                self.prompt_drafts.set(None);
                                self.prompt_prepared.set(None);
                                self.batch_prepared.set(None);
                                self.prepared.set(None);
                                self.model_choices.set(None);
                                if destination == ReceiptDestination::Prompt {
                                    let prefix = self.composer.peek().trim().to_owned();
                                    self.composer.set(
                                        [prefix, blocks.join("\n\n")]
                                            .into_iter()
                                            .filter(|s| !s.is_empty())
                                            .collect::<Vec<_>>()
                                            .join("\n\n"),
                                    );
                                    self.batch.set(None);
                                    self.input.set(None);
                                } else {
                                    self.input.set(None);
                                    self.batch.set(Some((
                                        "รายการจากใบเสร็จ · ตรวจภาพและรายละเอียดทุกใบก่อนบันทึก".into(),
                                        drafts,
                                    )));
                                }
                                self.guidance.set(format!(
                                    "อ่านได้ {successes} ใบ ตรวจช่องที่ขาดและยอดกับภาพก่อนบันทึก"
                                ));
                            }
                        }
                    }
                    Err(error) => self.notice.set(Some((true, error.to_string()))),
                }
            }
            self.scan_cancel.set(None);
            self.scan_progress.set(String::new());
            self.busy.set(false);
        });
    }

    pub fn clear_receipts(mut self) {
        if *self.busy.peek() {
            return;
        }
        self.receipts.set(Vec::new());
        let text = without_receipt_blocks(&self.composer.peek());
        self.composer.set(text);
        self.batch_prepared.set(None);
        self.prepared.set(None);
        if let Some((_, drafts)) = self.batch.write().as_mut() {
            drafts.retain(|d| d.input.receipt.is_none());
        }
        if self
            .batch
            .peek()
            .as_ref()
            .is_some_and(|(_, d)| d.is_empty())
        {
            self.batch.set(None);
            if *self.page.peek() == Page::Manual
                && let Some(today) = self.view.peek().as_ref().map(|v| v.today)
            {
                self.input.set(Some(EntryInput::empty(today)));
            }
        }
        self.guidance.set(String::new());
    }
    pub fn cancel_scan(self) {
        if let Some(cancel) = self.scan_cancel.peek().as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
    }
    pub fn send(mut self, command: Command) {
        if *self.busy.peek() {
            return;
        }
        self.busy.set(true);
        self.notice.set(None);
        let gateway = self.gateway.peek().clone();
        // A successful commit dismisses its dialog/preview. The task must belong
        // to the root, otherwise unmounting that child cancels the refresh and
        // leaves the entire app busy even though the transaction was saved.
        let created_account = matches!(&command, Command::CreateAccount { .. });
        let recurring_change = matches!(
            &command,
            Command::AddRecurring(_)
                | Command::SetCreditCycle { .. }
                | Command::CreateReceivable(_)
                | Command::ReceiveRepayment(_)
                | Command::SetRecurringInstallments { .. }
                | Command::StopRecurring { .. }
                | Command::PayRecurring(_)
                | Command::LinkRecurring { .. }
        );
        dioxus::dioxus_core::spawn_forever(async move {
            let result = gateway.0.request(command).await;
            match result {
                Ok(Response::Preferences(_)) => {}
                Ok(Response::ReceivableReview(review)) => self.receivable_review.set(Some(review)),
                Ok(Response::Dashboard(view)) => self.view.set(Some(view)),
                Ok(Response::Resolved(resolution)) => self.apply_resolution(resolution),
                Ok(Response::PreparedRecurringPayment(prepared)) => {
                    self.recurring_prepared.set(Some(prepared))
                }
                Ok(Response::PreparedPrompt(plan)) => self.prompt_prepared.set(Some(plan)),
                Ok(Response::Prepared(prepared)) => self.prepared.set(Some(prepared)),
                Ok(Response::PreparedBatch(prepared)) => self.batch_prepared.set(Some(prepared)),
                Ok(Response::AccountDeletion(deletion)) => {
                    self.account_deletion.set(Some(deletion))
                }
                Ok(Response::AccountDeleted(id)) => {
                    self.account_deletion.set(None);
                    self.batch_prepared.set(None);
                    if let Some((_, drafts)) = self.batch.write().as_mut() {
                        for draft in drafts {
                            if draft.input.account == Some(id) {
                                draft.input.account = None;
                            }
                            if draft.input.destination == Some(id) {
                                draft.input.destination = None;
                            }
                        }
                    }
                    // A draft referring to the removed account cannot be saved.
                    let uses_deleted = self.input.peek().as_ref().is_some_and(|input| {
                        input.account == Some(id) || input.destination == Some(id)
                    });
                    if uses_deleted {
                        self.prepared.set(None);
                        if let Some(input) = self.input.write().as_mut() {
                            if input.account == Some(id) {
                                input.account = None;
                            }
                            if input.destination == Some(id) {
                                input.destination = None;
                            }
                        }
                    }
                    // Never leave a stale dashboard available if refresh fails.
                    self.view.set(None);
                    match gateway.0.request(Command::Load).await {
                        Ok(Response::Dashboard(view)) => {
                            self.view.set(Some(view));
                            self.notice
                                .set(Some((false, "ลบบัญชีและรายการที่เกี่ยวข้องแล้ว".into())));
                        }
                        _ => self.notice.set(Some((
                            true,
                            "ลบแล้ว แต่โหลดภาพรวมไม่สำเร็จ กรุณากดโหลดใหม่ก่อนทำรายการต่อ".into(),
                        ))),
                    }
                }
                Ok(Response::PromptCommitted)
                | Ok(Response::Committed(_))
                | Ok(Response::CommittedBatch(_))
                | Ok(Response::RecurringChanged) => {
                    self.receivable_review.set(None);
                    self.repayment_form.set(false);
                    self.recurring_prepared.set(None);
                    if !recurring_change {
                        if !created_account {
                            self.prompt_drafts.set(None);
                            self.prompt_prepared.set(None);
                            self.batch.set(None);
                            self.guidance.set(String::new());
                            self.receipts.set(Vec::new());
                            self.composer.set(String::new());
                        }
                        self.batch_prepared.set(None);

                        self.account_form.set(false);
                        self.prepared.set(None);
                        self.input.set(None);
                    }
                    match gateway.0.request(Command::Load).await {
                        Ok(Response::Dashboard(view)) => {
                            self.view.set(Some(view));
                            self.notice.set(Some((false, "บันทึกในเครื่องแล้ว".into())));
                        }
                        _ => self.notice.set(Some((
                            true,
                            "บันทึกแล้ว แต่โหลดภาพรวมไม่สำเร็จ กรุณากดโหลดใหม่ก่อนทำรายการต่อ".into(),
                        ))),
                    }
                }
                Ok(Response::Csv(csv)) => match gateway.0.save_csv(csv).await {
                    Ok(Some(name)) => {
                        self.export_form.set(false);
                        self.notice.set(Some((false, format!("ส่งออกแล้ว: {name}"))));
                    }
                    Ok(None) => {}
                    Err(error) => self.notice.set(Some((true, error.to_string()))),
                },
                Err(error) => self.notice.set(Some((true, error.to_string()))),
            }
            self.busy.set(false);
        });
    }
    pub fn new_entry(mut self) {
        if *self.busy.peek() && self.model_operation.peek().is_none() {
            return;
        }
        let today = self.view.peek().as_ref().map(|v| v.today);
        if let Some(today) = today {
            self.prompt_drafts.set(None);
            self.prompt_prepared.set(None);
            self.batch.set(None);
            self.batch_prepared.set(None);
            // Cancel interpretation before opening an independent draft. The
            // awaiting task rejects late model output and releases busy itself.
            self.cancel_model();
            self.model_choices.set(None);

            self.input.set(Some(EntryInput::empty(today)));
            self.prepared.set(None);
            self.guidance.set(String::new());
            self.notice.set(None);
            self.receipts.set(Vec::new());
            self.composer.set(String::new());
            self.page.set(Page::Manual);
        }
    }
    pub fn update_entry(mut self, update: impl FnOnce(&mut EntryInput)) {
        if *self.busy.peek() {
            return;
        }
        self.notice.set(None);
        self.prepared.set(None);
        if let Some(input) = self.input.write().as_mut() {
            let previous_amount = input.amount.clone();
            update(input);
            if input.amount != previous_amount
                && let Some(receipt) = &mut input.receipt
            {
                receipt.reviewed = false;
            }
        }
    }
}

/// Release the form even if a native worker stops replying. Dropping a proposal
/// future never writes the ledger; cancellation also stops cooperative inference.
async fn await_model(
    future: crate::UiFuture<Response>,
    operation: &ModelOperation,
    limit: std::time::Duration,
) -> Result<Option<Response>, AppError> {
    let deadline = tokio::time::sleep(limit);
    tokio::pin!(future, deadline);
    loop {
        tokio::select! {
            biased;
            result = &mut future => return if operation.cancelled.load(Ordering::Relaxed) { Ok(None) } else { result.map(Some) },
            _ = &mut deadline => {
                operation.cancelled.store(true, Ordering::Relaxed);
                return Err(AppError::Input("AI ใช้เวลานานเกินไป ลองใหม่หรือใช้แบบฟอร์ม ยังไม่มีรายการถูกบันทึก".into()));
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                if operation.cancelled.load(Ordering::Relaxed) {
                    return Ok(None);
                }
            }
        }
    }
}

async fn await_receipt<T>(
    future: crate::UiFuture<T>,
    cancel: &AtomicBool,
    seconds: u64,
) -> Result<T, AppError> {
    let deadline = tokio::time::sleep(std::time::Duration::from_secs(seconds));
    tokio::pin!(future, deadline);
    loop {
        tokio::select! {
            result = &mut future => return result,
            _ = &mut deadline => {
                cancel.store(true, Ordering::Relaxed);
                return Err(AppError::Input("หมดเวลาอ่านใบเสร็จ กรุณาลองใหม่".into()));
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                if cancel.load(Ordering::Relaxed) { return Err(AppError::Input("ยกเลิกการอ่านใบเสร็จแล้ว".into())); }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::{UiFuture, UiGateway};
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    #[tokio::test]
    async fn stalled_model_times_out_and_cancels_native_work() {
        let operation = ModelOperation::default();
        let result = await_model(
            Box::pin(std::future::pending()),
            &operation,
            Duration::from_millis(20),
        )
        .await;
        assert!(result.is_err());
        assert!(operation.cancelled.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn cancel_does_not_wait_for_a_stalled_worker_or_accept_a_late_proposal() {
        let operation = ModelOperation::default();
        let control = operation.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            control.cancelled.store(true, Ordering::Relaxed);
        });
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            await_model(
                Box::pin(std::future::pending()),
                &operation,
                Duration::from_secs(90),
            ),
        )
        .await
        .expect("cancel must release the UI")
        .expect("cancelled result");
        assert!(result.is_none());
        let late = await_model(
            Box::pin(async { Ok(Response::Resolved(QuickResolution::Summary)) }),
            &operation,
            Duration::from_secs(90),
        )
        .await
        .expect("late result rejected");
        assert!(late.is_none());
    }

    struct DelayedRefresh {
        calls: Arc<AtomicUsize>,
    }
    impl UiGateway for DelayedRefresh {
        fn request(&self, command: Command) -> UiFuture<Response> {
            let calls = Arc::clone(&self.calls);
            Box::pin(async move {
                match command {
                    Command::CreateAccount { .. } => Ok(Response::Committed(CommitOutcome::Saved(
                        "00000000-0000-0000-0000-000000000001".parse()?,
                    ))),
                    Command::Load => {
                        calls.fetch_add(1, Ordering::SeqCst);
                        // The preview/dialog unmounts before the refresh completes.
                        tokio::time::sleep(Duration::from_millis(25)).await;
                        Ok(Response::Dashboard(dashboard(
                            LedgerState::default(),
                            "2026-09-20".parse()?,
                        )?))
                    }
                    _ => Err(AppError::Input("unexpected test command".into())),
                }
            })
        }
        fn save_csv(&self, _: CsvExport) -> UiFuture<Option<String>> {
            Box::pin(async { Ok(None) })
        }
        fn scan_receipt(
            &self,
            _: dioxus::html::FileData,
            _: Arc<AtomicBool>,
        ) -> UiFuture<(ReceiptImage, String)> {
            Box::pin(async { Err(AppError::Input("unexpected scan".into())) })
        }
    }

    fn use_test_state() -> UiState {
        let gateway = use_context::<Gateway>();
        UiState {
            gateway: use_signal(|| gateway),
            view: use_signal(|| None),
            page: use_signal(|| Page::Overview),
            busy: use_signal(|| false),
            notice: use_signal(|| None),
            account_form: use_signal(|| true),
            export_form: use_signal(|| false),
            account_deletion: use_signal(|| None),
            input: use_signal(|| None),
            prepared: use_signal(|| None),
            guidance: use_signal(String::new),
            composer: use_signal(String::new),
            receipts: use_signal(Vec::new),
            scan_progress: use_signal(String::new),
            scan_cancel: use_signal(|| None),
            model_operation: use_signal(|| None),
            model_choices: use_signal(|| None),
            prompt_drafts: use_signal(|| None),
            prompt_prepared: use_signal(|| None),
            batch: use_signal(|| None),
            batch_prepared: use_signal(|| None),
            recurring_prepared: use_signal(|| None),
            receivable_review: use_signal(|| None),
            repayment_form: use_signal(|| false),
            repayment_selection: use_signal(|| None),
        }
    }

    #[component]
    fn Harness() -> Element {
        let store = use_test_state();
        use_context_provider(|| store);
        let ready =
            store.view.read().is_some() && !*store.busy.read() && !*store.account_form.read();
        rsx! {
            if *store.account_form.read() { OriginatingChild {} }
            div { if ready { "refreshed-and-ready" } else { "pending" } }
        }
    }

    #[component]
    fn OriginatingChild() -> Element {
        let store = use_context::<UiState>();
        use_effect(move || {
            store.send(Command::CreateAccount {
                credit_cycle: None,
                name: "cash".into(),
                kind: ledger_domain::AccountKind::Cash,
                opening: "0".into(),
            })
        });
        rsx! { div { "originating-dialog" } }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn commit_refresh_survives_unmounting_the_originating_component() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut dom = VirtualDom::new(Harness);
        dom.insert_any_root_context(Box::new(Gateway(Arc::new(DelayedRefresh {
            calls: Arc::clone(&calls),
        }))));
        dom.rebuild_in_place();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                dom.wait_for_work().await;
                dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
                if dioxus_ssr::render(&dom).contains("refreshed-and-ready") {
                    break;
                }
            }
        })
        .await
        .expect("a committed entry must refresh and release busy after the child closes");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(!dioxus_ssr::render(&dom).contains("originating-dialog"));
    }

    #[component]
    fn BatchAccountHarness() -> Element {
        let store = use_test_state();
        use_effect(move || {
            store.apply_resolution(QuickResolution::Batch {
                source: "ซื้อไก่ทอด30บาท".into(),
                drafts: vec![ModelDraft {
                    input: EntryInput::empty("2026-09-24".parse().expect("date")),
                    guidance: "เพิ่มบัญชี".into(),
                }],
            });
            store.send(Command::CreateAccount {
                credit_cycle: None,
                name: "เงินสด".into(),
                kind: ledger_domain::AccountKind::Cash,
                opening: "0".into(),
            });
        });
        let ready = store.view.read().is_some() && !*store.busy.read();
        let retained = store
            .batch
            .read()
            .as_ref()
            .is_some_and(|(source, drafts)| source == "ซื้อไก่ทอด30บาท" && drafts.len() == 1);
        rsx! { div { if ready && retained { "batch-retained-after-account-creation" } else { "pending" } } }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn creating_a_missing_account_preserves_the_uncommitted_prompt_batch() {
        let mut dom = VirtualDom::new(BatchAccountHarness);
        dom.insert_any_root_context(Box::new(Gateway(Arc::new(DelayedRefresh {
            calls: Arc::new(AtomicUsize::new(0)),
        }))));
        dom.rebuild_in_place();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                dom.wait_for_work().await;
                dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
                if dioxus_ssr::render(&dom).contains("batch-retained-after-account-creation") {
                    break;
                }
            }
        })
        .await
        .expect("adding the missing account must not discard the prompt");
    }

    #[derive(Clone, Copy)]
    enum ManualStart {
        Idle,
        StalledModel,
        Saving,
    }

    struct ManualGateway {
        requests: Arc<AtomicUsize>,
        models: Arc<AtomicUsize>,
    }
    impl UiGateway for ManualGateway {
        fn request(&self, _: Command) -> UiFuture<Response> {
            self.requests.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Err(AppError::Input("unexpected request".into())) })
        }
        fn resolve_text(&self, _: String, _: ModelOperation) -> UiFuture<Response> {
            self.models.fetch_add(1, Ordering::SeqCst);
            Box::pin(std::future::pending())
        }
        fn save_csv(&self, _: CsvExport) -> UiFuture<Option<String>> {
            Box::pin(async { Ok(None) })
        }
        fn scan_receipt(
            &self,
            _: dioxus::html::FileData,
            _: Arc<AtomicBool>,
        ) -> UiFuture<(ReceiptImage, String)> {
            Box::pin(async { Err(AppError::Input("unexpected scan".into())) })
        }
    }

    #[component]
    fn ManualHarness() -> Element {
        let mut store = use_test_state();
        let start = use_context::<ManualStart>();
        let today = "2026-09-21".parse().expect("fixture date");
        use_effect(move || {
            store.view.set(Some(
                dashboard(LedgerState::default(), today).expect("fixture"),
            ));
            if !matches!(start, ManualStart::Saving) {
                store.batch.set(Some(("ซื้อข้าว30บาท".into(), vec![])));
            }
            match start {
                ManualStart::Idle => {}
                ManualStart::StalledModel => store.resolve_text("กาแฟ 80".into()),
                ManualStart::Saving => store.busy.set(true),
            }
            store.new_entry();
        });
        let ready = *store.page.read() == Page::Manual
            && store.input.read().as_ref() == Some(&EntryInput::empty(today))
            && !*store.busy.read()
            && store.model_choices.read().is_none()
            && store.batch.read().is_none()
            && store.batch_prepared.read().is_none()
            && store.prepared.read().is_none();
        let protected = matches!(start, ManualStart::Saving)
            && *store.busy.read()
            && *store.page.read() == Page::Overview
            && store.input.read().is_none();
        rsx! { div { if ready { "manual-ready" } else if protected { "save-protected" } else { "pending" } } }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn manual_entry_bypasses_ai_and_recovers_from_a_stalled_model() {
        for start in [
            ManualStart::Idle,
            ManualStart::StalledModel,
            ManualStart::Saving,
        ] {
            let requests = Arc::new(AtomicUsize::new(0));
            let models = Arc::new(AtomicUsize::new(0));
            let mut dom = VirtualDom::new(ManualHarness);
            dom.insert_any_root_context(Box::new(start));
            dom.insert_any_root_context(Box::new(Gateway(Arc::new(ManualGateway {
                requests: Arc::clone(&requests),
                models: Arc::clone(&models),
            }))));
            dom.rebuild_in_place();
            let expected = if matches!(start, ManualStart::Saving) {
                "save-protected"
            } else {
                "manual-ready"
            };
            tokio::time::timeout(Duration::from_secs(2), async {
                loop {
                    dom.wait_for_work().await;
                    dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
                    if dioxus_ssr::render(&dom).contains(expected) {
                        break;
                    }
                }
            })
            .await
            .expect(
                "manual fallback must open without waiting for AI and must respect an active save",
            );
            assert_eq!(requests.load(Ordering::SeqCst), 0);
            assert_eq!(
                models.load(Ordering::SeqCst),
                usize::from(matches!(start, ManualStart::StalledModel))
            );
        }
    }
    #[component]
    fn RecurringBatchHarness() -> Element {
        let view = use_context::<Dashboard>();
        let mut store = use_test_state();
        let today = view.today;
        store.batch = use_signal(|| {
            Some((
                "prompt".into(),
                vec![ModelDraft {
                    input: EntryInput {
                        amount: "90.00".into(),
                        note: "สมาชิก".into(),
                        ..EntryInput::empty(today)
                    },
                    guidance: String::new(),
                }],
            ))
        });
        use_context_provider(|| store);
        let (source, drafts) = store.batch.read().clone().expect("batch");
        rsx! { crate::batch_entry::BatchReview { source, drafts, view } }
    }

    #[test]
    fn selecting_recurring_in_prompt_updates_the_review_without_replacing_amount() {
        use dioxus::dioxus_core::{AttributeValue, Mutation};
        use ledger_domain::*;
        use std::{any::Any, rc::Rc};
        let plan = RecurringExpense::new(
            "00000000-0000-0000-0000-000000000002".parse().expect("id"),
            AccountName::new("สมาชิก").expect("name"),
            PositiveMoney::new("100.00".parse().expect("money")).expect("positive"),
            Category::OtherExpense,
            None,
            MonthlyDue::new(1, "2026-09".parse().expect("month"))
                .expect("due")
                .with_installments(Some(2))
                .expect("count"),
        )
        .expect("plan");
        let mut state = LedgerState::default();
        state.recurring.push(plan.clone());
        let view = dashboard(state, "2026-09-24".parse().expect("date")).expect("view");
        set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
        let mut dom = VirtualDom::new(RecurringBatchHarness);
        dom.insert_any_root_context(Box::new(view));
        dom.insert_any_root_context(Box::new(Gateway(Arc::new(DelayedRefresh {
            calls: Arc::new(AtomicUsize::new(0)),
        }))));
        let changes = dom.rebuild_to_vec();
        let select = changes
            .edits
            .iter()
            .find_map(|edit| match edit {
                Mutation::SetAttribute {
                    name: "id",
                    value: AttributeValue::Text(value),
                    id,
                    ..
                } if value == "batch-recurring-0-plan" => Some(*id),
                _ => None,
            })
            .expect("plan dropdown");
        let data = dioxus::html::SerializedFormData::new(plan.id().to_string(), vec![]);
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>,
            true,
        );
        dom.runtime().handle_event("change", event, select);
        dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("batch-recurring-0-period"), "{html}");
        assert!(html.contains("value=\"90.00\""), "{html}");
        assert!(!html.contains("ยังขาด: หมวดหมู่"), "{html}");
    }
    struct ReceiptGateway {
        calls: Arc<AtomicUsize>,
        stalled: bool,
    }
    impl UiGateway for ReceiptGateway {
        fn request(&self, _: Command) -> UiFuture<Response> {
            self.calls.fetch_add(1000, Ordering::SeqCst);
            Box::pin(async {
                Err(AppError::Input(
                    "upload must not commit or invoke AI".into(),
                ))
            })
        }
        fn save_csv(&self, _: CsvExport) -> UiFuture<Option<String>> {
            Box::pin(async { Ok(None) })
        }
        fn scan_receipt(
            &self,
            file: dioxus::html::FileData,
            _: Arc<AtomicBool>,
        ) -> UiFuture<(ReceiptImage, String)> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let stalled = self.stalled;
            Box::pin(async move {
                if stalled {
                    return std::future::pending().await;
                }
                if file.name() == "bad.png" {
                    return Err(AppError::Input("รูปเสีย".into()));
                }
                Ok((
                    ReceiptImage::new(b"\x89PNG\r\n\x1a\n".to_vec())?,
                    "Coffee 80.00\nGrand Total 80.00".into(),
                ))
            })
        }
    }
    #[derive(Clone, Copy)]
    struct ReceiptTest {
        manual: bool,
        cancel: bool,
        oversized: bool,
    }
    #[component]
    fn ReceiptHarness() -> Element {
        let mut store = use_test_state();
        let config = use_context::<ReceiptTest>();
        use_effect(move || {
            let today = "2026-09-24".parse().expect("date");
            store.view.set(Some(
                dashboard(LedgerState::default(), today).expect("view"),
            ));
            store.page.set(if config.manual {
                Page::Manual
            } else {
                Page::Chat
            });
            store.composer.set("กาแฟ 30".into());
            if config.manual {
                let mut input = EntryInput::empty(today);
                input.amount = "30".into();
                store.input.set(Some(input));
            }
            let files = ["first.png", "bad.png", "second.png"]
                .map(|name| {
                    dioxus::html::FileData::new(dioxus::html::SerializedFileData {
                        path: name.into(),
                        size: if config.oversized {
                            MAX_RECEIPT_BATCH_BYTES
                        } else {
                            20
                        },
                        last_modified: 0,
                        content_type: Some("image/png".into()),
                        contents: None,
                    })
                })
                .to_vec();
            store.scan_receipts(
                files,
                if config.manual {
                    ReceiptDestination::Manual
                } else {
                    ReceiptDestination::Prompt
                },
            );
            if config.cancel {
                spawn(async move {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    store.cancel_scan();
                });
            }
        });
        let composer = store.composer.read().clone();
        let count = store.receipts.read().len();
        let errors = store
            .receipts
            .read()
            .iter()
            .filter(|a| a.review.is_err())
            .count();
        let batch = store
            .batch
            .read()
            .as_ref()
            .map(|(_, d)| d.len())
            .unwrap_or(0);
        let manual = *store.page.read() == Page::Manual;
        let busy = *store.busy.read();
        let notice = store.notice.read().clone();
        rsx! { div { "busy={busy};images={count};errors={errors};drafts={batch};manual={manual};{composer}" if let Some((_, notice)) = notice { "{notice}" } } }
    }
    async fn scan_harness(config: ReceiptTest) -> (String, usize) {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut dom = VirtualDom::new(ReceiptHarness);
        dom.insert_any_root_context(Box::new(config));
        dom.insert_any_root_context(Box::new(Gateway(Arc::new(ReceiptGateway {
            calls: calls.clone(),
            stalled: config.cancel,
        }))));
        dom.rebuild_in_place();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                dom.wait_for_work().await;
                dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
                let html = dioxus_ssr::render(&dom);
                if html.contains("busy=false")
                    && (html.contains("images=3")
                        || html.contains("หยุดอ่าน")
                        || html.contains("128 MB"))
                {
                    break;
                }
            }
        })
        .await
        .expect("scan releases UI");
        (dioxus_ssr::render(&dom), calls.load(Ordering::SeqCst))
    }
    #[tokio::test(flavor = "current_thread")]
    async fn multiple_uploads_append_prompt_and_report_each_failed_image_without_saving() {
        let (html, calls) = scan_harness(ReceiptTest {
            manual: false,
            cancel: false,
            oversized: false,
        })
        .await;
        assert_eq!(calls, 3);
        assert!(html.contains("images=3;errors=1;drafts=0"), "{html}");
        assert!(html.contains("กาแฟ 30"));
        assert!(html.contains("[ใบเสร็จ 1]"));
        assert!(html.contains("[ใบเสร็จ 3]"));
        assert!(!html.contains("[ใบเสร็จ 2]"));
    }
    #[tokio::test(flavor = "current_thread")]
    async fn manual_upload_opens_drafts_and_preserves_the_existing_unsaved_entry() {
        let (html, _) = scan_harness(ReceiptTest {
            manual: true,
            cancel: false,
            oversized: false,
        })
        .await;
        assert!(
            html.contains("images=3;errors=1;drafts=3;manual=true"),
            "{html}"
        );
        assert!(!html.contains("[ใบเสร็จ"));
    }
    #[tokio::test(flavor = "current_thread")]
    async fn cancelling_stalled_ocr_releases_ui_and_keeps_original_text() {
        let (html, calls) = scan_harness(ReceiptTest {
            manual: false,
            cancel: true,
            oversized: false,
        })
        .await;
        assert_eq!(calls, 1);
        assert!(
            html.contains("images=0;errors=0;drafts=0;manual=false;กาแฟ 30"),
            "{html}"
        );
    }
    #[tokio::test(flavor = "current_thread")]
    async fn oversized_batches_are_rejected_before_platform_work() {
        let (html, calls) = scan_harness(ReceiptTest {
            manual: false,
            cancel: false,
            oversized: true,
        })
        .await;
        assert_eq!(calls, 0);
        assert!(html.contains("128 MB"));
    }
    #[component]
    fn ReceiptDropHarness() -> Element {
        let mut store = use_test_state();
        use_context_provider(|| store);
        let view = use_hook(|| {
            dashboard(LedgerState::default(), "2026-09-24".parse().expect("date")).expect("view")
        });
        let initial = view.clone();
        use_hook(move || {
            store.view.set(Some(initial));
            store.page.set(Page::Chat);
        });
        rsx! { crate::entry::QuickEntryPage { view } }
    }
    #[tokio::test(flavor = "current_thread")]
    async fn dropping_multiple_files_on_composer_runs_ocr_and_preserves_editable_blocks() {
        use dioxus::dioxus_core::Mutation;
        use dioxus::html::{PlatformEventData, set_event_converter};
        use std::{any::Any, rc::Rc};
        let calls = Arc::new(AtomicUsize::new(0));
        let mut dom = VirtualDom::new(ReceiptDropHarness);
        dom.insert_any_root_context(Box::new(Gateway(Arc::new(ReceiptGateway {
            calls: calls.clone(),
            stalled: false,
        }))));
        dom.insert_any_root_context(Box::new(crate::HostInfo {
            art: Arc::new(crate::ArtAssets::bundled()),
            isolated: true,
            receipt_ocr_available: true,
            preview_label: "test",
        }));
        set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
        let edits = dom.rebuild_to_vec();
        let target = edits
            .edits
            .iter()
            .find_map(|e| match e {
                Mutation::NewEventListener { name, id } if name == "drop" => Some(*id),
                _ => None,
            })
            .expect("composer drop target");
        let initial = dioxus_ssr::render(&dom);
        assert!(initial.contains("multiple"));
        assert!(!initial.contains("receipt-scanner"));
        let data = dioxus::html::SerializedDragData {
            mouse: Default::default(),
            data_transfer: dioxus::html::SerializedDataTransfer {
                items: vec![],
                effect_allowed: "all".into(),
                drop_effect: "copy".into(),
                files: ["first.png", "second.png"]
                    .map(|name| dioxus::html::SerializedFileData {
                        path: name.into(),
                        size: 20,
                        last_modified: 0,
                        content_type: Some("image/png".into()),
                        contents: None,
                    })
                    .to_vec(),
            },
        };
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>,
            true,
        );
        dom.runtime().handle_event("drop", event, target);
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                dom.wait_for_work().await;
                dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
                if dioxus_ssr::render(&dom).contains("[ใบเสร็จ 2]") {
                    break;
                }
            }
        })
        .await
        .expect("drop produces both blocks");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(dioxus_ssr::render(&dom).contains("[ใบเสร็จ 1]"));
    }
}
