use ledger_domain::DomainError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StorageError {
    #[error("ไฟล์สำรองไม่ถูกต้อง หรือสร้างจากแอปรุ่นที่ยังไม่รองรับ")]
    InvalidBackup,
    #[error("กู้คืนได้เฉพาะผู้ใช้ที่ยังไม่มีข้อมูล กรุณาสร้างผู้ใช้ใหม่เพื่อย้ายข้อมูลเข้า")]
    RestoreNeedsEmptyLedger,
    #[error("จำนวนเหรียญในพอร์ตเปลี่ยนไปแล้ว กรุณาโหลดใหม่ก่อนบันทึก")]
    CryptoHoldingsChanged,
    #[error("บัญชีนี้ผูกกับรายจ่ายประจำ กรุณาหยุดแผนเดิมและเลือกบัญชีจ่ายอื่นก่อนเปลี่ยนเป็นจำนวนเหรียญ")]
    CryptoRecurringAccount,
    #[error("รอบบัตรถูกกำหนดไปแล้วหรือบัญชีเปลี่ยนไป กรุณาโหลดข้อมูลใหม่")]
    CreditCycleChanged,
    #[error(
        "Currency is locked after the first account, bill, or receivable. Use a new ledger for a different currency."
    )]
    CurrencyLocked,
    #[error("ข้อมูลเปลี่ยนหลังจากตรวจ prompt กรุณาตรวจทั้งชุดใหม่ก่อนยืนยัน")]
    PromptChanged,
    #[error("ข้อมูลลูกหนี้เปลี่ยนไปแล้ว กรุณาโหลดใหม่และตรวจอีกครั้ง")]
    ReceivableChanged,
    #[error("ยอดชำระเกินเงินต้นค้าง หรือการยกเลิกจะทำให้ยอดหนี้ไม่ถูกต้อง กรุณาตรวจรายการชำระก่อน")]
    ReceivableOverpayment,
    #[error("วันที่รับชำระต้องไม่ก่อนวันที่ตั้งยอดหนี้")]
    ReceivableDate,
    #[error("บัญชีนี้มีรายการให้ยืมหรือรับชำระหนี้ กรุณาเก็บบัญชีไว้เพื่อรักษาประวัติลูกหนี้")]
    ReceivableAccountInUse,
    #[error("เพิ่มสัญญาลูกหนี้ได้สูงสุด 500 รายการ")]
    ReceivableLimit,
    #[error("จำนวนงวดใหม่ต้องครอบคลุมงวดที่เคยบันทึกจ่ายไว้ ประวัติการจ่ายจะไม่ถูกตัดทิ้ง")]
    InstallmentsConflict,
    #[error("แผนรายจ่ายเปลี่ยนไปแล้ว กรุณาโหลดใหม่แล้วตรวจอีกครั้ง")]
    RecurringChanged,
    #[error("มีประวัติบันทึกจ่ายตั้งแต่งวดที่เลือก กรุณาเลือกเดือนเริ่มใช้การแก้ไขหลังงวดที่เคยบันทึกจ่าย")]
    RecurringEditPaid,
    #[error("งวดนี้จ่ายแล้ว หรือรายการจ่ายถูกผูกกับงวดอื่นแล้ว กรุณาโหลดใหม่")]
    RecurringPaid,
    #[error("เพิ่มแผนรายจ่ายประจำได้สูงสุด 500 แผน")]
    RecurringLimit,
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
