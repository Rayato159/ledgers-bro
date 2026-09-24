//! Reproducible synthetic screenshot data. Refuses to open an existing directory.
use ledger_application::*;
use ledger_domain::*;
use ledger_infrastructure::{RandomIds, SqliteLedger};
use std::{error::Error, path::PathBuf};

struct DemoClock;
impl Clock for DemoClock {
    fn today(&self) -> Result<EntryDate, AppError> {
        Ok("2026-09-24".parse()?)
    }
}
type Demo = LedgerApplication<SqliteLedger, DemoClock, RandomIds>;
fn view(app: &mut Demo) -> Result<Dashboard, Box<dyn Error>> {
    match app.execute(Command::Load)? {
        Response::Dashboard(v) => Ok(v),
        _ => Err("expected dashboard".into()),
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let directory = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("Pass a new, empty demo directory")?;
    std::fs::create_dir(&directory)?;
    let mut app = LedgerApplication::new(
        SqliteLedger::open(&directory.join("ledger.sqlite3"))?,
        DemoClock,
        RandomIds,
    );
    for (name, kind, opening) in [
        ("Cash", AccountKind::Cash, "3500"),
        ("Everyday bank", AccountKind::Bank, "85000"),
        ("Travel fund", AccountKind::Bank, "24000"),
        ("Credit card", AccountKind::CreditCard, "1800"),
    ] {
        app.execute(Command::CreateAccount {
            name: name.into(),
            kind,
            opening: opening.into(),
        })?;
    }
    let accounts = view(&mut app)?.accounts;
    let bank = accounts
        .iter()
        .find(|a| a.account.name().as_str() == "Everyday bank")
        .ok_or("missing demo account")?
        .account
        .id();
    let cash = accounts
        .iter()
        .find(|a| a.account.name().as_str() == "Cash")
        .ok_or("missing demo account")?
        .account
        .id();
    for (month, income, spending) in [
        (4, "28000", "24500"),
        (5, "32000", "26000"),
        (6, "30000", "22500"),
        (7, "35000", "25000"),
        (8, "32000", "21500"),
        (9, "36000", "18900"),
    ] {
        for (kind, category, amount, note) in [
            (
                TransactionKind::Income,
                Category::Salary,
                income,
                "Monthly salary",
            ),
            (
                TransactionKind::Expense,
                Category::OtherExpense,
                spending,
                "Monthly living expenses",
            ),
        ] {
            let input = EntryInput {
                kind,
                account: Some(bank),
                amount: amount.into(),
                category: Some(category),
                date: format!("2026-{month:02}-01"),
                note: note.into(),
                ..EntryInput::empty("2026-09-24".parse()?)
            };
            if let Response::Prepared(p) = app.execute(Command::Preview(input))? {
                app.execute(Command::Commit(p))?;
            } else {
                return Err("expected preview".into());
            }
        }
    }
    for (name, amount, day, category, installments) in [
        ("Apartment rent", "6500", "28", Category::Rent, None),
        (
            "Phone installment",
            "1200",
            "25",
            Category::Supplies,
            Some("12"),
        ),
        (
            "Music subscription",
            "149",
            "27",
            Category::OtherExpense,
            None,
        ),
    ] {
        app.execute(Command::AddRecurring(RecurringInput {
            name: name.into(),
            amount: amount.into(),
            day: day.into(),
            start: "2026-09".into(),
            account: Some(bank),
            category: Some(category),
            installments: installments.map(Into::into),
        }))?;
    }
    for (debtor, description, total, count, day, repaid) in [
        (
            "Jamie",
            "Laptop loan",
            "12000",
            Some("6"),
            Some("15"),
            "4000",
        ),
        (
            "Alex",
            "Shared holiday",
            "4500",
            Some("3"),
            Some("28"),
            "1500",
        ),
        ("Sam", "Concert tickets", "2400", None, None, "0"),
    ] {
        let input = ReceivableInput {
            debtor: debtor.into(),
            description: description.into(),
            total: total.into(),
            opened: "2026-07-01".into(),
            start: "2026-07".into(),
            day: day.map(Into::into),
            installments: count.map(Into::into),
            source: None,
        };
        let Response::ReceivableReview(ReceivableReview::New(p)) =
            app.execute(Command::PreviewReceivable(input))?
        else {
            return Err("expected receivable preview".into());
        };
        let loan = p.loan.clone();
        app.execute(Command::CreateReceivable(*p))?;
        if repaid != "0" {
            let Response::ReceivableReview(ReceivableReview::Payment(p)) =
                app.execute(Command::PreviewRepayment(RepaymentInput {
                    receivable: loan.id(),
                    account: Some(bank),
                    principal: repaid.into(),
                    interest: "0".into(),
                    date: "2026-09-20".into(),
                }))?
            else {
                return Err("expected repayment preview".into());
            };
            app.execute(Command::ReceiveRepayment(p))?;
        }
    }
    for (amount, category, note) in [
        ("80", Category::Snacks, "Morning coffee"),
        ("240", Category::Food, "Lunch with friends"),
        ("590", Category::Supplies, "Desk essentials"),
    ] {
        let input = EntryInput {
            account: Some(cash),
            amount: amount.into(),
            category: Some(category),
            note: note.into(),
            ..EntryInput::empty("2026-09-24".parse()?)
        };
        if let Response::Prepared(p) = app.execute(Command::Preview(input))? {
            app.execute(Command::Commit(p))?;
        }
    }
    println!("Synthetic ledger created in {}", directory.display());
    Ok(())
}
