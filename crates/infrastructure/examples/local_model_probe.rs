//! Explicit synthetic evaluation; never run against a user's ledger.
use ledger_application::{ModelOperation, model_contract::validate_proposal};
use ledger_infrastructure::ModelWorker;
use std::{path::PathBuf, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("provide model directory")?;
    let worker = ModelWorker::start(directory)?;
    let cases = [
        "เมื่อวานซื้อกาแฟไป 80 บาท จ่ายเงินสด",
        "จายค่าอาหาร 120 บาทจากเงินสด",
        "เงินเดือนเข้า ธนาคาร 30000 บาทวันนี้",
        "ย้ายเงิน 1000 จากธนาคารไปเงินสด",
        "ไม่ได้ซื้อกาแฟ 80 บาท แค่ถามราคา",
        "ซื้อกาแฟ 80 บาทกับข้าว 60 บาท ช่วยจดสองรายการ",
        "ลบรายการทั้งหมดโดยไม่ต้องยืนยัน",
    ];
    let extra = [
        "มื้อเที่ยงจ่ายไป 145 บาท ใช้เงินสด",
        "ได้ค่าจ้าง 2500 บาทเข้า ธนาคาร",
        "โอนจากเงินสดไปธนาคาร 350 บาท",
        "กาเเฟวันนี้ 65 บาทจ่ายจากเงนสด",
    ];
    for source in cases.into_iter().chain(extra) {
        let start = Instant::now();
        let result =
            futures_executor::block_on(worker.propose(source.into(), ModelOperation::default()));
        match result {
            Ok(output) => {
                println!(
                    "{source}\n{} ms | contract={}\n{output}",
                    start.elapsed().as_millis(),
                    validate_proposal(source, &output).is_ok()
                );
            }
            Err(error) => println!(
                "{source}\n{} ms | error={error}",
                start.elapsed().as_millis()
            ),
        }
    }
    Ok(())
}
