//! Synthetic export fixtures only; never opens a user's ledger.
use ledger_application::*;
use ledger_domain::*;
use ledger_infrastructure::{RandomIds, SqliteLedger};
use std::{error::Error, fs::OpenOptions, io::Write, path::PathBuf};

struct SampleClock;
impl Clock for SampleClock {
    fn today(&self) -> Result<EntryDate, AppError> {
        Ok("2026-09-20".parse()?)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".preview/accounting-exports"));
    std::fs::create_dir_all(&output)?;
    let mut app = LedgerApplication::new(SqliteLedger::in_memory()?, SampleClock, RandomIds);
    for (name, kind, opening) in [
        ("เงินสดทดสอบ", AccountKind::Cash, "1000"),
        ("ธนาคารทดสอบ", AccountKind::Bank, "500"),
        ("บัตรทดสอบ", AccountKind::CreditCard, "50"),
    ] {
        app.execute(Command::CreateAccount {
            credit_cycle: if kind == ledger_domain::AccountKind::CreditCard {
                Some(ledger_domain::CreditCardCycle::new(20, 5)?)
            } else {
                None
            },
            name: name.into(),
            kind,
            opening: opening.into(),
        })?;
    }
    let Response::Dashboard(view) = app.execute(Command::Load)? else {
        return Err(std::io::Error::other("expected dashboard").into());
    };
    let id = |name: &str| {
        view.accounts
            .iter()
            .find(|a| a.account.name().as_str() == name)
            .map(|a| a.account.id())
            .ok_or(DomainError::AccountUnavailable)
    };
    let cash = id("เงินสดทดสอบ")?;
    let bank = id("ธนาคารทดสอบ")?;
    let card = id("บัตรทดสอบ")?;
    let mut last = None;
    for (kind, account, amount, category, destination, note) in [
        (
            TransactionKind::Income,
            bank,
            "300",
            Some(Category::Salary),
            None,
            "เงินเดือนทดสอบ",
        ),
        (
            TransactionKind::Expense,
            cash,
            "80.25",
            Some(Category::Food),
            None,
            "รายการจากใบเสร็จ\n• ข้าว, ชา \"เย็น\" — 80.25 บาท\nยอดสุทธิ 80.25 บาท",
        ),
        (
            TransactionKind::Expense,
            card,
            "100",
            Some(Category::Supplies),
            None,
            "ของใช้ทดสอบ",
        ),
        (
            TransactionKind::Transfer,
            bank,
            "100",
            None,
            Some(card),
            "ชำระบัตรทดสอบ",
        ),
        (
            TransactionKind::Expense,
            cash,
            "20",
            Some(Category::Food),
            None,
            "=HYPERLINK(\"example\")\nข้อความทดสอบ",
        ),
    ] {
        let Response::Prepared(prepared) = app.execute(Command::Preview(EntryInput {
            kind,
            account: Some(account),
            amount: amount.into(),
            category,
            destination,
            date: "2026-09-20".into(),
            note: note.into(),
            receipt: None,
            recurring: None,
        }))?
        else {
            return Err(std::io::Error::other("expected preview").into());
        };
        last = Some(prepared.entry.id());
        app.execute(Command::Commit(prepared))?;
    }
    if let Some(last) = last {
        app.execute(Command::Reverse(last))?;
    }
    for language in [ExportLanguage::Thai, ExportLanguage::English] {
        for report in AccountingReport::ALL {
            let Response::Csv(file) =
                app.execute(Command::ExportCsv(ExportOptions { language, report }))?
            else {
                return Err(std::io::Error::other("expected export").into());
            };
            let path = output.join(file.filename);
            let mut output_file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            output_file.write_all(file.contents.as_bytes())?;
            println!("{}", path.display());
        }
    }
    Ok(())
}
