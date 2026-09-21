use ledger_domain::DomainError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StorageError {
    #[error(transparent)]
    Rule(#[from] DomainError),
    #[error("เปิดหรือบันทึกฐานข้อมูลไม่ได้ กรุณาลองใหม่ ข้อมูลเดิมจะไม่ถูกแทนที่")]
    Unavailable,
    #[error("ข้อมูลไม่ผ่านการตรวจสอบความสมบูรณ์ หยุดบันทึกเพื่อรักษาข้อมูลเดิม")]
    Corrupt,
    #[error("ฐานข้อมูลนี้มาจากแอปรุ่นใหม่กว่า กรุณาอัปเดตแอป")]
    NewerDatabase,
    #[error("รหัสการบันทึกนี้ถูกใช้กับข้อมูลอื่นแล้ว กรุณาตรวจรายการใหม่")]
    SubmissionConflict,
    #[error("ข้อมูลบัญชีหรือรายการเปลี่ยนไปแล้ว ยังไม่ได้ลบ กรุณาตรวจสอบรายการใหม่ก่อนยืนยัน")]
    DeletionChanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AppError {
    #[error(transparent)]
    Rule(#[from] DomainError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("{0}")]
    Input(String),
    #[error("แอปกำลังทำงาน กรุณารอสักครู่แล้วลองอีกครั้ง")]
    Busy,
    #[error("ส่วนจัดเก็บข้อมูลหยุดทำงาน กรุณาปิดแล้วเปิดแอปใหม่")]
    WorkerStopped,
}
