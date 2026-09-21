//! Android document selection and on-device OCR. No accounting authority.
use crate::platform::on_activity;
use ledger_application::{AppError, MAX_OCR_TEXT_BYTES, MAX_RECEIPT_BYTES, ReceiptImage};
use ledger_ui::UiFuture;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI64, Ordering},
    },
    time::{Duration, Instant},
};
static NEXT_SCAN: AtomicI64 = AtomicI64::new(1);

pub fn pick(cancel: Arc<AtomicBool>) -> UiFuture<Option<(ReceiptImage, String)>> {
    Box::pin(async move {
        let id = NEXT_SCAN.fetch_add(1, Ordering::Relaxed);
        let (started, vm, activity) = on_activity(move |env, activity| {
            let started = env
                .call_method(activity, "beginReceipt", "(J)Z", &[id.into()])?
                .z()?;
            Ok((started, env.get_java_vm()?, env.new_global_ref(activity)?))
        })
        .await?;
        if !started {
            return Err(AppError::Input(
                "ตัวอ่านใบเสร็จยังไม่พร้อม รอสักครู่แล้วลองใหม่".into(),
            ));
        }
        // Poll volatile native results on a worker attached to the JVM. A SAF
        // picker backgrounds the Activity; queued Wry UI dispatches can stall
        // across that transition. Only opening the picker requires the UI thread.
        let (send, receive) = futures_channel::oneshot::channel();
        std::thread::Builder::new()
            .name("receipt-result".into())
            .spawn(move || {
                let result = (|| {
                    let mut env = vm
                        .attach_current_thread()
                        .map_err(|_| AppError::WorkerStopped)?;
                    let result = collect_result(&mut env, activity.as_obj(), id, &cancel);
                    if result.is_err() {
                        let _ = env.exception_clear();
                        let _ = env.call_method(
                            activity.as_obj(),
                            "cancelReceipt",
                            "(J)V",
                            &[id.into()],
                        );
                        let _ = env.exception_clear();
                    }
                    result
                })();
                let _ = send.send(result);
            })
            .map_err(|_| AppError::WorkerStopped)?;
        receive.await.map_err(|_| AppError::WorkerStopped)?
    })
}

fn collect_result(
    env: &mut jni::JNIEnv<'_>,
    activity: &jni::objects::JObject<'_>,
    id: i64,
    cancel: &AtomicBool,
) -> Result<Option<(ReceiptImage, String)>, AppError> {
    let mut processing_started = None;
    let picker_deadline = Instant::now() + Duration::from_secs(300);
    loop {
        if cancel.load(Ordering::Relaxed)
            || Instant::now() >= picker_deadline
            || processing_started
                .is_some_and(|start: Instant| start.elapsed() > Duration::from_secs(60))
        {
            return Err(AppError::Input(
                if cancel.load(Ordering::Relaxed) {
                    "ยกเลิกการอ่านใบเสร็จแล้ว"
                } else {
                    "หมดเวลาอ่านภาพ กรุณาลองใหม่"
                }
                .into(),
            ));
        }
        std::thread::sleep(Duration::from_millis(150));
        let (state, text, bytes, error) = env
            .with_local_frame(16, |env| -> jni::errors::Result<_> {
                let args = [id.into()];
                let state = env
                    .call_method(activity, "receiptState", "(J)I", &args)?
                    .i()?;
                let mut text = String::new();
                let mut bytes = Vec::new();
                let mut error = 0;
                if state == 3 {
                    let value = env
                        .call_method(activity, "receiptText", "(J)Ljava/lang/String;", &args)?
                        .l()?;
                    text = env.get_string(&value.into())?.into();
                    let value = env
                        .call_method(activity, "receiptPreview", "(J)[B", &args)?
                        .l()?;
                    bytes = env.convert_byte_array(jni::objects::JByteArray::from(value))?;
                    env.call_method(activity, "clearReceipt", "(J)V", &args)?;
                } else if state == 5 {
                    error = env
                        .call_method(activity, "receiptError", "(J)I", &args)?
                        .i()?;
                }
                Ok((state, text, bytes, error))
            })
            .map_err(|_| AppError::Input("ติดต่อระบบอ่านใบเสร็จไม่สำเร็จ ลองใหม่อีกครั้ง".into()))?;
        match state {
            1 => {}
            2 => {
                processing_started.get_or_insert_with(Instant::now);
            }
            3 => {
                if bytes.len() > MAX_RECEIPT_BYTES || text.len() > MAX_OCR_TEXT_BYTES {
                    return Err(AppError::Input(
                        "รูปหรือข้อความยาวเกินไป กรุณาครอปเฉพาะใบเสร็จ".into(),
                    ));
                }
                if cancel.load(Ordering::Relaxed) {
                    return Ok(None);
                }
                return Ok(Some((ReceiptImage::new(bytes)?, text)));
            }
            4 => return Ok(None),
            _ => return Err(AppError::Input(error_message(error).into())),
        }
    }
}
fn error_message(code: i32) -> &'static str {
    match code {
        1 => "เลือกรูป JPG, PNG หรือ HEIC/HEIF ไม่เกิน 32 MB",
        2 => "อ่านภาพไม่ได้ หรือภาพใหญ่เกิน 50 ล้านพิกเซล ลองส่งเป็น JPG หรือครอปภาพก่อน",
        3 => "ชุดอ่านภาษาไทย/อังกฤษไม่พร้อม กรุณาติดตั้งแอปรุ่นล่าสุด",
        4 => "ยังอ่านข้อความไม่เจอ ลองถ่ายใบเสร็จให้ตรงและมีแสงเพียงพอ",
        5 => "การรับรูปใบเสร็จรุ่นนี้ต้องใช้ Android 9 ขึ้นไป",
        6 => "หมดเวลาอ่านภาพ ลองครอปเฉพาะใบเสร็จแล้วเลือกอีกครั้ง",
        8 => "Android เครื่องนี้ถอดรหัสภาพนี้ไม่ได้ ลองแปลงเป็น JPG/PNG แล้วเลือกใหม่",
        _ => "อ่านใบเสร็จไม่สำเร็จ ลองเลือกรูปใหม่ แอปไม่ได้ส่งภาพขึ้น Cloud",
    }
}
