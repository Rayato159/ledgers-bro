//! Android ASR is an input adapter, independent of accounting and the quick parser.
use crate::platform::on_activity;
use dioxus::mobile::wry;
use ledger_application::AppError;
use ledger_ui::{UiFuture, VoiceEvent, VoiceSessionId};
use std::time::Duration;

pub fn begin(id: VoiceSessionId, method: &'static str) -> UiFuture<()> {
    Box::pin(on_activity(move |env, activity| {
        env.call_method(activity, method, "(J)V", &[id.get().into()])?;
        Ok(())
    }))
}

pub fn poll(id: VoiceSessionId) -> UiFuture<VoiceEvent> {
    Box::pin(async move {
        tokio::time::sleep(Duration::from_millis(150)).await;
        // One main-thread dispatch provides a consistent state/result snapshot.
        let (state, transcript, error) = on_activity(move |env, activity| {
            let args = [id.get().into()];
            let state = env
                .call_method(activity, "voiceState", "(J)I", &args)?
                .i()?;
            let mut transcript = String::new();
            let mut error = 0;
            if state == 5 {
                let value = env
                    .call_method(activity, "voiceText", "(J)Ljava/lang/String;", &args)?
                    .l()?;
                transcript = env.get_string(&value.into())?.into();
            } else if state == 7 {
                error = env
                    .call_method(activity, "voiceError", "(J)I", &args)?
                    .i()?;
            }
            Ok((state, transcript, error))
        })
        .await?;
        match state {
            1 => Ok(VoiceEvent::Preparing),
            2 => Ok(VoiceEvent::Permission),
            3 => Ok(VoiceEvent::Listening),
            4 => Ok(VoiceEvent::Processing),
            5 => Ok(VoiceEvent::Finished(transcript)),
            6 => Ok(VoiceEvent::Cancelled),
            7 if error == 1002 => Ok(VoiceEvent::ModelRequired),
            7 => Err(AppError::Input(error_message(error).into())),
            8 => Ok(VoiceEvent::ModelDownloadRequested),
            9 => Ok(VoiceEvent::ModelReady),
            _ => Err(AppError::Input("ตัวถอดเสียงตอบกลับไม่ถูกต้อง ลองใหม่อีกครั้ง".into())),
        }
    })
}

pub fn control(id: VoiceSessionId, method: &'static str) {
    wry::prelude::dispatch(move |env, activity, _| {
        if env
            .call_method(activity, method, "(J)V", &[id.get().into()])
            .is_err()
        {
            let _ = env.exception_clear();
        }
    });
}

fn error_message(code: i32) -> &'static str {
    match code {
        1001 => "เครื่องนี้ไม่มีตัวถอดเสียงบนเครื่องที่ใช้ได้ ต้องใช้ Android 12 ขึ้นไปและบริการที่รองรับ พิมพ์รายการแทนได้",
        1010 => "ขอดาวน์โหลดโมเดลภาษาไทยไม่สำเร็จ ตรวจอินเทอร์เน็ตและบริการเสียงของ Android แล้วลองใหม่",
        1002 | 13 => {
            "เครื่องนี้ยังไม่มีโมเดลถอดเสียงภาษาไทยออฟไลน์ที่พร้อมใช้ ตรวจภาษาออฟไลน์ในการตั้งค่าบริการเสียงของ Android แล้วลองใหม่ หรือพิมพ์รายการแทน"
        }
        1008 | 12 => {
            "บริการเสียงของเครื่องนี้ไม่รองรับภาษาไทยแบบออฟไลน์ ลองบน Android เครื่องอื่นที่รองรับ หรือพิมพ์รายการแทน แอปไม่สลับไปใช้ Cloud"
        }
        1009 => {
            "โมเดลภาษาไทยออฟไลน์ของ Android กำลังรอติดตั้ง ให้ติดตั้งเสร็จแล้วลองปุ่มพูดอีกครั้ง ระหว่างนี้พิมพ์รายการได้"
        }
        1003 => "บริการเสียงของเครื่องนี้ยืนยันการรองรับภาษาไทยออฟไลน์ไม่ได้ ลองอัปเดตบริการเสียงหรือพิมพ์รายการแทน",
        1004 | 9 => "ยังไม่ได้อนุญาตไมโครโฟน กดปุ่มพูดอีกครั้ง หรือเปิดสิทธิ์ไมโครโฟนในการตั้งค่าแอป Android",
        1005 | 1 => "หมดเวลารับเสียง ลองพูดประโยคสั้น ๆ อีกครั้ง",
        1006 | 8 => "ไมโครโฟนยังไม่พร้อม ปิดหน้าต่างขอสิทธิ์หรือการใช้ไมค์อื่นก่อน แล้วลองอีกครั้ง",
        6 | 7 => "ยังฟังไม่ชัด ลองพูดทีละรายการ เช่น กาแฟ 80 หรือพิมพ์รายการแทน",
        3 => "เปิดไมโครโฟนไม่ได้ ตรวจว่าไมค์ไม่ได้ถูกปิดในการตั้งค่าความเป็นส่วนตัวของ Android",
        _ => "ถอดเสียงบนเครื่องไม่สำเร็จ ลองอีกครั้งหรือพิมพ์รายการ แอปไม่ได้สลับไปใช้ Cloud",
    }
}
