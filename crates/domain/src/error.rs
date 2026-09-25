use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("จำนวนเหรียญต้องไม่ติดลบ: BTC รองรับ 8 และ SOL รองรับ 9 ตำแหน่งทศนิยม")]
    InvalidCryptoQuantity,
    #[error("ราคาคริปโตหรือเวลาราคาไม่ถูกต้อง กรุณาลองอัปเดตใหม่")]
    InvalidCryptoPrice,
    #[error("พอร์ตจำนวนเหรียญใช้แก้ยอด BTC/SOL โดยตรง ไม่ใช้รับจ่ายหรือโอนเงินบาท")]
    CryptoCashEntry,
    #[error("ตรวจประเภทเงินได้และยอดภาษี: เงินได้ก่อนหัก + VAT − หัก ณ ที่จ่าย − รายการหักอื่น ต้องเท่ากับยอดรับจริง")]
    InvalidIncomeTax,
    #[error("บัตรเครดิตต้องกำหนดวันตัดรอบและวันครบกำหนดชำระเป็นวันที่ 1–31")]
    InvalidCreditCycle,
    #[error("Unsupported ledger currency. Choose THB, USD, EUR, GBP, AUD, CAD, SGD, or CNY.")]
    InvalidCurrency,
    #[error("เรื่องหนี้ต้องมี 1–300 ตัวอักษร วันเริ่มเก็บต้องไม่ก่อนตั้งหนี้ และยอดต้องเพียงพอสำหรับจำนวนงวด")]
    InvalidReceivable,
    #[error("จำนวนเงินต้องเป็นตัวเลขและมีทศนิยมไม่เกิน 2 ตำแหน่ง")]
    InvalidMoney,
    #[error("จำนวนเงินเกินขอบเขตที่รองรับ")]
    MoneyOverflow,
    #[error("จำนวนเงินต้องมากกว่าศูนย์")]
    NonPositiveAmount,
    #[error("ชื่อจำเป็นต้องมี 1–60 ตัวอักษร และไม่มีอักขระควบคุม")]
    InvalidName,
    #[error("รายละเอียดต้องไม่เกิน 20,000 ตัวอักษร และไม่มีอักขระควบคุมที่ไม่รองรับ")]
    InvalidNote,
    #[error("ชื่อรายการใบเสร็จต้องมี 1–120 ตัวอักษร และยอดต้องไม่ติดลบ ยกเว้นการปัดเศษ")]
    InvalidReceiptLine,
    #[error("ใบเสร็จต้องมีสินค้า/บริการอย่างน้อยหนึ่งรายการ และรวมไม่เกิน 100 บรรทัด")]
    InvalidReceiptLines,
    #[error("รวมรายการได้ {calculated} บาท แต่ยอดสุทธิเป็น {total} บาท กรุณาตรวจและแก้ให้ตรง")]
    ReceiptTotalMismatch {
        calculated: crate::Money,
        total: crate::Money,
    },
    #[error("ภาษี/ค่าบริการที่รวมในราคาแล้วต้องไม่เกินยอดสุทธิ")]
    InvalidIncludedCharge,
    #[error("รหัสข้อมูลไม่ถูกต้อง")]
    InvalidId,
    #[error("วันที่ไม่ถูกต้อง ใช้ YYYY-MM-DD (ค.ศ.)")]
    InvalidDate,
    #[error("วันครบกำหนดต้องเป็นวันที่ 1–31 และเดือนสิ้นสุดต้องไม่ก่อนเดือนเริ่มต้น")]
    InvalidRecurring,
    #[error("จำนวนงวดต้องเป็นจำนวนเต็ม 1–1,200 หรือเลือกไม่กำหนดจำนวนงวด")]
    InvalidInstallments,
    #[error("เพิ่มได้สูงสุด 100 บัญชี รวมบัญชีที่เก็บเข้าคลัง")]
    AccountLimit,
    #[error("มีบัญชีชื่อนี้อยู่แล้ว")]
    DuplicateAccountName,
    #[error("ไม่พบบัญชี หรือบัญชีนี้ถูกเก็บเข้าคลังแล้ว")]
    AccountUnavailable,
    #[error("บัญชีต้นทางและปลายทางต้องต่างกัน")]
    SameAccountTransfer,
    #[error("รายการบัญชีไม่สมดุล")]
    UnbalancedJournal,
    #[error("รายการนี้ถูกยกเลิกแล้ว หรือไม่สามารถยกเลิกได้")]
    InvalidReversal,
    #[error("หมวดหนี้ต้องใช้การโอนไปบัญชีหนี้ แยกดอกเบี้ยเป็นค่าใช้จ่าย")]
    DebtNeedsTransfer,
    #[error("หมวดนี้ไม่ตรงกับชนิดรายการ")]
    InvalidCategory,
}
