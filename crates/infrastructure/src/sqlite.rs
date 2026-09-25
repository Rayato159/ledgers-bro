use crate::wire::StoredKind;
use ledger_application::UserPreferences;
use ledger_application::{
    AccountDeletion, CommitOutcome, LedgerRepository, LedgerState, StorageError, validate_append,
};
use ledger_domain::*;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::Duration,
};

const APPLICATION_ID: i64 = 1_279_414_863;
const SCHEMA_VERSION: i64 = 8;

pub struct SqliteLedger {
    connection: Connection,
}
impl SqliteLedger {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let connection = Connection::open(path).map_err(database_error)?;
        Self::initialize(connection)
    }
    pub fn in_memory() -> Result<Self, StorageError> {
        Self::initialize(Connection::open_in_memory().map_err(database_error)?)
    }
    fn initialize(mut connection: Connection) -> Result<Self, StorageError> {
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(database_error)?;
        connection
            .execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF;")
            .map_err(database_error)?;
        // The write lock also serializes two app instances creating a fresh database.
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let version: i64 = tx
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(database_error)?;
        let app: i64 = tx
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .map_err(database_error)?;
        if version > SCHEMA_VERSION {
            return Err(StorageError::NewerDatabase);
        }
        if version == 0 {
            let tables: i64 = tx.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'", [], |row| row.get(0)).map_err(database_error)?;
            if tables != 0 || (app != 0 && app != APPLICATION_ID) {
                return Err(StorageError::Corrupt);
            }
            tx.execute_batch(include_str!("../migrations/001_ledger.sql"))
                .map_err(database_error)?;
        } else if app != APPLICATION_ID {
            return Err(StorageError::Corrupt);
        }
        if version < 2 {
            tx.execute_batch(include_str!("../migrations/002_recurring.sql"))
                .map_err(database_error)?;
        }
        if version < 3 {
            tx.execute_batch(include_str!("../migrations/003_installments.sql"))
                .map_err(database_error)?;
        }
        if version < 4 {
            tx.execute_batch(include_str!("../migrations/004_receivables.sql"))
                .map_err(database_error)?;
        }
        if version < 5 {
            tx.execute_batch(include_str!("../migrations/005_prompt_submissions.sql"))
                .map_err(database_error)?;
        }
        if version < 6 {
            tx.execute_batch(include_str!("../migrations/006_currency.sql"))
                .map_err(database_error)?;
        }
        if version < 7 {
            tx.execute_batch(include_str!("../migrations/007_preferences.sql"))
                .map_err(database_error)?;
        }
        if version < 8 {
            tx.execute_batch(include_str!("../migrations/008_credit_cycles.sql"))
                .map_err(database_error)?;
        }
        if version < SCHEMA_VERSION {
            read_state(&tx)?;
        }
        tx.commit().map_err(database_error)?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(database_error)?;
        connection
            .pragma_update(None, "synchronous", "FULL")
            .map_err(database_error)?;
        Ok(Self { connection })
    }
}

impl LedgerRepository for SqliteLedger {
    fn set_credit_cycle(
        &mut self,
        expected: &Account,
        cycle: CreditCardCycle,
    ) -> Result<(), StorageError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let updated =
            ledger_application::configure_credit_cycle(&read_state(&tx)?, expected, cycle)?;
        tx.execute(
            "UPDATE accounts SET closing_day=?1,payment_day=?2 WHERE id=?3",
            params![
                cycle.closing_day(),
                cycle.payment_day(),
                updated.id().to_string()
            ],
        )
        .map_err(database_error)?;
        tx.commit().map_err(database_error)
    }
    fn preferences(&mut self) -> Result<UserPreferences, StorageError> {
        self.connection
            .query_row(
                "SELECT english,dark,primary_color,gradient FROM user_preferences WHERE id=1",
                [],
                |r| {
                    Ok(UserPreferences {
                        english: r.get(0)?,
                        dark: r.get(1)?,
                        primary_color: r.get(2)?,
                        gradient: r.get(3)?,
                    })
                },
            )
            .map_err(database_error)
    }
    fn set_preferences(&mut self, p: UserPreferences) -> Result<(), StorageError> {
        self.connection.execute("UPDATE user_preferences SET english=?1,dark=?2,primary_color=?3,gradient=?4 WHERE id=1", params![p.english,p.dark,p.primary_color,p.gradient]).map_err(database_error)?;
        Ok(())
    }

    fn set_thai_tax_enabled(&mut self, enabled: bool) -> Result<(), StorageError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let current = read_state(&tx)?;
        if enabled && current.currency != Currency::Thb {
            return Err(DomainError::InvalidCurrency.into());
        }
        tx.execute(
            "UPDATE ledger_settings SET thai_tax_enabled=?1 WHERE id=1",
            [enabled],
        )
        .map_err(database_error)?;
        tx.commit().map_err(database_error)
    }
    fn set_currency(&mut self, currency: Currency) -> Result<(), StorageError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let current = read_state(&tx)?;
        if current.currency_locked && current.currency != currency {
            return Err(StorageError::CurrencyLocked);
        }
        tx.execute("UPDATE ledger_settings SET currency=?1,thai_tax_enabled=CASE WHEN ?1='THB' THEN thai_tax_enabled ELSE 0 END WHERE id=1", [currency.code()]).map_err(database_error)?;
        tx.commit().map_err(database_error)
    }
    fn commit_prompt(&mut self, plan: &ledger_application::PromptPlan) -> Result<(), StorageError> {
        use sha2::{Digest, Sha256};
        let fingerprint = format!("{:x}", Sha256::digest(format!("{plan:?}").as_bytes()));
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let previous: Option<String> = tx
            .query_row(
                "SELECT fingerprint FROM prompt_submissions WHERE id=?1",
                [plan.id().to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)?;
        if let Some(previous) = previous {
            return if previous == fingerprint {
                Ok(())
            } else {
                Err(StorageError::SubmissionConflict)
            };
        }
        let before = read_state(&tx)?;
        plan.validate_current(&before)?;
        let after = plan.after();
        ledger_application::validate_receivables(after)?;
        for entry in before
            .entries
            .iter()
            .rev()
            .filter(|e| !after.entries.iter().any(|a| a.id() == e.id()))
        {
            tx.execute(
                "DELETE FROM postings WHERE entry_id=?1",
                [entry.id().to_string()],
            )
            .map_err(database_error)?;
            tx.execute(
                "DELETE FROM journal_entries WHERE id=?1",
                [entry.id().to_string()],
            )
            .map_err(database_error)?;
        }
        for account in before
            .accounts
            .iter()
            .filter(|a| !after.accounts.iter().any(|b| b.id() == a.id()))
        {
            tx.execute(
                "DELETE FROM accounts WHERE id=?1",
                [account.id().to_string()],
            )
            .map_err(database_error)?;
        }
        for account in after
            .accounts
            .iter()
            .filter(|a| !before.accounts.iter().any(|b| b.id() == a.id()))
        {
            tx.execute(
                "INSERT INTO accounts(id,name,name_key,kind,archived,closing_day,payment_day) VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    account.id().to_string(),
                    account.name().as_str(),
                    account.name().key(),
                    account.kind().code(),
                    account.is_archived(),
                    account.credit_cycle().map(CreditCardCycle::closing_day),
                    account.credit_cycle().map(CreditCardCycle::payment_day)
                ],
            )
            .map_err(database_error)?;
        }
        for loan in after
            .receivables
            .iter()
            .filter(|p| !before.receivables.iter().any(|b| b.id() == p.id()))
        {
            tx.execute("INSERT INTO receivables(id,debtor,description,total_minor,opened,start_month,day,installments) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)", params![loan.id().to_string(),loan.debtor().as_str(),loan.description().as_str(),loan.total().money().minor(),loan.opened().to_string(),loan.start().to_string(),loan.day(),loan.installments()]).map_err(database_error)?;
        }
        for schedule in &after.recurring {
            if let Some(old) = before.recurring.iter().find(|s| s.id() == schedule.id()) {
                if old != schedule {
                    tx.execute("UPDATE recurring_expenses SET installments=?1, stopped_from=?2, account_id=?3 WHERE id=?4", params![schedule.due().installments(),schedule.stopped_from().map(|m| m.to_string()),schedule.account().map(|id| id.to_string()),schedule.id().to_string()]).map_err(database_error)?;
                }
            } else {
                tx.execute("INSERT INTO recurring_expenses(id,name,amount_minor,category,account_id,day,start_month,stopped_from,installments) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![schedule.id().to_string(),schedule.name().as_str(),schedule.amount().money().minor(),schedule.category().code(),schedule.account().map(|id| id.to_string()),schedule.due().day(),schedule.due().start().to_string(),schedule.stopped_from().map(|m| m.to_string()),schedule.due().installments()]).map_err(database_error)?;
            }
        }
        for prepared in plan.entries() {
            insert_entry(&tx, &prepared.entry, prepared.submission)?;
        }
        for settlement in after
            .settlements
            .iter()
            .filter(|s| !before.settlements.contains(s))
        {
            insert_settlement(
                &tx,
                settlement.recurring,
                settlement.month,
                settlement.entry,
            )?;
        }
        read_state(&tx)?;
        tx.execute(
            "INSERT INTO prompt_submissions(id,fingerprint) VALUES(?1,?2)",
            params![plan.id().to_string(), fingerprint],
        )
        .map_err(database_error)?;
        tx.commit().map_err(database_error)
    }
    fn create_receivable(
        &mut self,
        prepared: &ledger_application::PreparedReceivable,
    ) -> Result<CommitOutcome, StorageError> {
        let loan = &prepared.loan;
        let opening = &prepared.opening;
        if opening.recurring.is_some()
            || !matches!(opening.entry.kind(), EntryKind::ReceivableOpening { receivable, .. } | EntryKind::Lending { receivable, .. } if *receivable == loan.id())
        {
            return Err(StorageError::Corrupt);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let mut state = read_state(&tx)?;
        if let Some(outcome) = existing_submission(&tx, &opening.entry, opening.submission)? {
            if !state.receivables.contains(loan) {
                return Err(StorageError::SubmissionConflict);
            }
            return Ok(outcome);
        }
        state.receivables.push(loan.clone());
        validate_append(&state, &opening.entry)?;
        tx.execute("INSERT INTO receivables(id,debtor,description,total_minor,opened,start_month,day,installments) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)", params![loan.id().to_string(), loan.debtor().as_str(), loan.description().as_str(), loan.total().money().minor(), loan.opened().to_string(), loan.start().to_string(), loan.day(), loan.installments()]).map_err(database_error)?;
        insert_entry(&tx, &opening.entry, opening.submission)?;
        tx.commit().map_err(database_error)?;
        Ok(CommitOutcome::Saved(opening.entry.id()))
    }
    fn set_recurring_installments(
        &mut self,
        expected: &RecurringExpense,
        count: Option<u32>,
    ) -> Result<(), StorageError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let state = read_state(&tx)?;
        ledger_application::changed_installments(&state, expected, count)?;
        tx.execute(
            "UPDATE recurring_expenses SET installments=?1 WHERE id=?2",
            params![count, expected.id().to_string()],
        )
        .map_err(database_error)?;
        tx.commit().map_err(database_error)
    }
    fn add_recurring(&mut self, schedule: &RecurringExpense) -> Result<(), StorageError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let state = read_state(&tx)?;
        ledger_application::validate_new_recurring(&state, schedule)?;
        tx.execute("INSERT INTO recurring_expenses(id,name,amount_minor,category,account_id,day,start_month,stopped_from,installments) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![schedule.id().to_string(),schedule.name().as_str(),schedule.amount().money().minor(),schedule.category().code(),schedule.account().map(|id| id.to_string()),schedule.due().day(),schedule.due().start().to_string(),schedule.stopped_from().map(|m| m.to_string()),schedule.due().installments()]).map_err(database_error)?;
        tx.commit().map_err(database_error)
    }
    fn stop_recurring(
        &mut self,
        expected: &RecurringExpense,
        month: Month,
    ) -> Result<(), StorageError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let state = read_state(&tx)?;
        if !state.recurring.contains(expected) || expected.stopped_from().is_some() {
            return Err(StorageError::RecurringChanged);
        }
        expected.clone().stop_from(month)?;
        tx.execute(
            "UPDATE recurring_expenses SET stopped_from=?1 WHERE id=?2",
            params![month.to_string(), expected.id().to_string()],
        )
        .map_err(database_error)?;
        tx.commit().map_err(database_error)
    }
    fn pay_recurring(
        &mut self,
        prepared: &ledger_application::PreparedRecurringPayment,
    ) -> Result<CommitOutcome, StorageError> {
        let mut payment = prepared.payment.clone();
        let link = ledger_application::PreparedRecurringLink {
            schedule: prepared.schedule.clone(),
            month: prepared.month,
        };
        if payment
            .recurring
            .as_ref()
            .is_some_and(|existing| existing != &link)
        {
            return Err(StorageError::SubmissionConflict);
        }
        payment.recurring = Some(link);
        self.commit_batch(&[payment])?
            .into_iter()
            .next()
            .ok_or(StorageError::Corrupt)
    }
    fn link_recurring(
        &mut self,
        expected: &RecurringExpense,
        month: Month,
        entry: EntryId,
    ) -> Result<(), StorageError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let state = read_state(&tx)?;
        let entry = state
            .entries
            .iter()
            .find(|e| e.id() == entry)
            .ok_or(StorageError::RecurringChanged)?;
        if state
            .settlements
            .iter()
            .any(|s| s.recurring == expected.id() && s.month == month && s.entry == entry.id())
        {
            return Ok(());
        }
        ledger_application::validate_recurring_settlement(&state, expected, month, entry)?;
        insert_settlement(&tx, expected.id(), month, entry.id())?;
        tx.commit().map_err(database_error)
    }
    fn commit_batch(
        &mut self,
        entries: &[ledger_application::PreparedEntry],
    ) -> Result<Vec<CommitOutcome>, StorageError> {
        if entries.is_empty() || entries.len() > ledger_application::MAX_BATCH_ENTRIES {
            return Err(StorageError::Corrupt);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let mut state = read_state(&tx)?;
        let mut outcomes = Vec::new();
        let mut submissions = BTreeSet::new();
        let mut ids = BTreeSet::new();
        for prepared in entries {
            if !submissions.insert(prepared.submission)
                || !ids.insert(prepared.entry.id())
                || matches!(
                    prepared.entry.kind(),
                    EntryKind::Opening { .. }
                        | EntryKind::ReceivableOpening { .. }
                        | EntryKind::Lending { .. }
                        | EntryKind::Reversal { .. }
                )
            {
                return Err(StorageError::SubmissionConflict);
            }
            if let Some(outcome) = existing_submission(&tx, &prepared.entry, prepared.submission)? {
                if let Some(link) = &prepared.recurring
                    && !state.settlements.iter().any(|s| {
                        s.recurring == link.schedule.id()
                            && s.month == link.month
                            && s.entry == prepared.entry.id()
                    })
                {
                    return Err(StorageError::SubmissionConflict);
                }
                outcomes.push(outcome);
            } else {
                validate_append(&state, &prepared.entry)?;
                if let Some(link) = &prepared.recurring {
                    ledger_application::validate_recurring_settlement(
                        &state,
                        &link.schedule,
                        link.month,
                        &prepared.entry,
                    )?;
                }
                insert_entry(&tx, &prepared.entry, prepared.submission)?;
                if let Some(link) = &prepared.recurring {
                    insert_settlement(&tx, link.schedule.id(), link.month, prepared.entry.id())?;
                    state
                        .settlements
                        .retain(|s| !(s.recurring == link.schedule.id() && s.month == link.month));
                    state
                        .settlements
                        .push(ledger_application::RecurringSettlement {
                            recurring: link.schedule.id(),
                            month: link.month,
                            entry: prepared.entry.id(),
                        });
                }
                state.entries.push(prepared.entry.clone());
                outcomes.push(CommitOutcome::Saved(prepared.entry.id()));
            }
        }
        tx.commit().map_err(database_error)?;
        Ok(outcomes)
    }
    fn delete_account(&mut self, deletion: &AccountDeletion) -> Result<(), StorageError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        deletion.validate_current(&read_state(&tx)?)?;
        for id in deletion.entry_ids() {
            tx.execute("DELETE FROM postings WHERE entry_id=?1", [id.to_string()])
                .map_err(database_error)?;
            let deleted = tx
                .execute("DELETE FROM journal_entries WHERE id=?1", [id.to_string()])
                .map_err(database_error)?;
            if deleted != 1 {
                return Err(StorageError::Corrupt);
            }
        }
        let deleted = tx
            .execute(
                "DELETE FROM accounts WHERE id=?1",
                [deletion.account().id().to_string()],
            )
            .map_err(database_error)?;
        if deleted != 1 {
            return Err(StorageError::Corrupt);
        }
        tx.commit().map_err(database_error)?;
        Ok(())
    }
    fn snapshot(&mut self) -> Result<LedgerState, StorageError> {
        let tx = self.connection.transaction().map_err(database_error)?;
        let state = read_state(&tx)?;
        tx.commit().map_err(database_error)?;
        Ok(state)
    }
    fn create_account(
        &mut self,
        account: &Account,
        opening: &JournalEntry,
        submission: SubmissionId,
    ) -> Result<CommitOutcome, StorageError> {
        if !matches!(opening.kind(), EntryKind::Opening { account: id, .. } if *id == account.id())
        {
            return Err(StorageError::Corrupt);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        if let Some(outcome) = existing_submission(&tx, opening, submission)? {
            return Ok(outcome);
        }
        let mut state = read_state(&tx)?;
        account.ensure_can_add(&state.accounts)?;
        state.accounts.push(account.clone());
        validate_append(&state, opening)?;
        tx.execute(
            "INSERT INTO accounts(id,name,name_key,kind,archived,closing_day,payment_day) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![
                account.id().to_string(),
                account.name().as_str(),
                account.name().key(),
                account.kind().code(),
                account.is_archived(),
                account.credit_cycle().map(CreditCardCycle::closing_day),
                account.credit_cycle().map(CreditCardCycle::payment_day)
            ],
        )
        .map_err(database_error)?;
        insert_entry(&tx, opening, submission)?;
        tx.commit().map_err(database_error)?;
        Ok(CommitOutcome::Saved(opening.id()))
    }
    fn commit(
        &mut self,
        entry: &JournalEntry,
        submission: SubmissionId,
    ) -> Result<CommitOutcome, StorageError> {
        if matches!(
            entry.kind(),
            EntryKind::Opening { .. }
                | EntryKind::ReceivableOpening { .. }
                | EntryKind::Lending { .. }
        ) {
            return Err(StorageError::Corrupt);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        if let Some(outcome) = existing_submission(&tx, entry, submission)? {
            return Ok(outcome);
        }
        validate_append(&read_state(&tx)?, entry)?;
        insert_entry(&tx, entry, submission)?;
        tx.commit().map_err(database_error)?;
        Ok(CommitOutcome::Saved(entry.id()))
    }
}

fn existing_submission(
    connection: &Connection,
    entry: &JournalEntry,
    submission: SubmissionId,
) -> Result<Option<CommitOutcome>, StorageError> {
    let saved = connection
        .query_row(
            "SELECT id,effective_date,note,payload FROM journal_entries WHERE submission_id=?1",
            [submission.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    if let Some((id, date, note, payload)) = saved {
        if date != entry.date().to_string()
            || note != entry.note().as_str()
            || payload != StoredKind::encode(entry)?
        {
            return Err(StorageError::SubmissionConflict);
        }
        return Ok(Some(CommitOutcome::AlreadySaved(id.parse()?)));
    }
    Ok(None)
}

fn insert_entry(
    connection: &Connection,
    entry: &JournalEntry,
    submission: SubmissionId,
) -> Result<(), StorageError> {
    let reverses = match entry.kind() {
        EntryKind::Reversal { original } => Some(original.to_string()),
        _ => None,
    };
    connection.execute("INSERT INTO journal_entries(id,submission_id,effective_date,note,payload,reverses) VALUES(?1,?2,?3,?4,?5,?6)", params![entry.id().to_string(), submission.to_string(), entry.date().to_string(), entry.note().as_str(), StoredKind::encode(entry)?, reverses]).map_err(database_error)?;
    for (ordinal, posting) in entry.postings().iter().enumerate() {
        let (account, book) = target_columns(posting.target());
        connection.execute("INSERT INTO postings(entry_id,ordinal,account_id,system_book,amount_minor) VALUES(?1,?2,?3,?4,?5)", params![entry.id().to_string(), ordinal as i64, account, book, posting.amount().minor()]).map_err(database_error)?;
    }
    Ok(())
}

fn target_columns(target: PostingTarget) -> (Option<String>, Option<&'static str>) {
    match target {
        PostingTarget::Account(id) => (Some(id.to_string()), None),
        PostingTarget::System(book) => (
            None,
            Some(match book {
                SystemBook::Receivable => "receivable",
                SystemBook::Equity => "equity",
                SystemBook::Income => "income",
                SystemBook::Expense => "expense",
            }),
        ),
    }
}

fn read_state(connection: &Connection) -> Result<LedgerState, StorageError> {
    let mut accounts_query = connection
        .prepare("SELECT id,name,name_key,kind,archived,closing_day,payment_day FROM accounts ORDER BY name_key,id")
        .map_err(database_error)?;
    let mut accounts_rows = accounts_query.query([]).map_err(database_error)?;
    let mut accounts = Vec::new();
    while let Some(row) = accounts_rows.next().map_err(database_error)? {
        let id: String = row.get(0).map_err(database_error)?;
        let name = AccountName::new(&row.get::<_, String>(1).map_err(database_error)?)
            .map_err(|_| StorageError::Corrupt)?;
        let key: String = row.get(2).map_err(database_error)?;
        if key != name.key() {
            return Err(StorageError::Corrupt);
        }
        let kind: String = row.get(3).map_err(database_error)?;
        let mut account = Account::restore(
            id.parse().map_err(|_| StorageError::Corrupt)?,
            name,
            AccountKind::from_code(&kind).map_err(|_| StorageError::Corrupt)?,
            row.get(4).map_err(database_error)?,
        );
        let closing: Option<u32> = row.get(5).map_err(database_error)?;
        let payment: Option<u32> = row.get(6).map_err(database_error)?;
        match (closing, payment) {
            (Some(closing), Some(payment)) => {
                let cycle =
                    CreditCardCycle::new(closing, payment).map_err(|_| StorageError::Corrupt)?;
                account = account
                    .with_credit_cycle(cycle)
                    .map_err(|_| StorageError::Corrupt)?;
            }
            (None, None) => {}
            _ => return Err(StorageError::Corrupt),
        }
        accounts.push(account);
    }
    if accounts.len() > MAX_ACCOUNTS {
        return Err(StorageError::Corrupt);
    }
    let mut query = connection
        .prepare(
            "SELECT id,effective_date,note,payload,reverses FROM journal_entries ORDER BY sequence",
        )
        .map_err(database_error)?;
    let mut rows = query.query([]).map_err(database_error)?;
    let mut lookup = BTreeMap::new();
    let mut reversed = BTreeSet::new();
    let mut entries = Vec::new();
    while let Some(row) = rows.next().map_err(database_error)? {
        let id: String = row.get(0).map_err(database_error)?;
        let date: String = row.get(1).map_err(database_error)?;
        let note: String = row.get(2).map_err(database_error)?;
        let payload: String = row.get(3).map_err(database_error)?;
        let reverses: Option<String> = row.get(4).map_err(database_error)?;
        let stored: StoredKind =
            serde_json::from_str(&payload).map_err(|_| StorageError::Corrupt)?;
        let entry = stored
            .restore(
                id.parse().map_err(|_| StorageError::Corrupt)?,
                date.parse().map_err(|_| StorageError::Corrupt)?,
                Note::new(&note).map_err(|_| StorageError::Corrupt)?,
                &lookup,
            )
            .map_err(|_| StorageError::Corrupt)?;
        match entry.kind() {
            EntryKind::Reversal { original }
                if reverses.as_deref() == Some(original.to_string().as_str())
                    && reversed.insert(*original) => {}
            EntryKind::Reversal { .. } => return Err(StorageError::Corrupt),
            _ if reverses.is_some() => return Err(StorageError::Corrupt),
            _ => {}
        }
        let mut p_query = connection.prepare("SELECT account_id,system_book,amount_minor FROM postings WHERE entry_id=?1 ORDER BY ordinal").map_err(database_error)?;
        let persisted = p_query
            .query_map([&id], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        if persisted.len() != entry.postings().len() {
            return Err(StorageError::Corrupt);
        }
        for (actual, expected) in persisted.iter().zip(entry.postings()) {
            let (account, book) = target_columns(expected.target());
            if actual.0 != account
                || actual.1.as_deref() != book
                || actual.2 != expected.amount().minor()
            {
                return Err(StorageError::Corrupt);
            }
            if let PostingTarget::Account(id) = expected.target()
                && !accounts.iter().any(|a| a.id() == id)
            {
                return Err(StorageError::Corrupt);
            }
        }
        lookup.insert(entry.id(), entry.clone());
        entries.push(entry);
    }
    let (recurring, settlements) = read_recurring(connection, &accounts, &entries)?;
    let (currency_code, currency_locked, thai_tax_enabled): (String, bool, bool) = connection
        .query_row(
            "SELECT currency,currency_locked,thai_tax_enabled FROM ledger_settings WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(database_error)?;
    let state = LedgerState {
        currency: Currency::from_code(&currency_code).map_err(|_| StorageError::Corrupt)?,
        currency_locked,
        thai_tax_enabled,
        receivables: read_receivables(connection)?,
        accounts,
        entries,
        recurring,
        settlements,
    };
    ledger_application::validate_receivables(&state).map_err(|_| StorageError::Corrupt)?;
    Ok(state)
}

fn read_receivables(connection: &Connection) -> Result<Vec<Receivable>, StorageError> {
    let mut query = connection.prepare("SELECT id,debtor,description,total_minor,opened,start_month,day,installments FROM receivables ORDER BY opened,id").map_err(database_error)?;
    let mut rows = query.query([]).map_err(database_error)?;
    let mut result = Vec::new();
    while let Some(row) = rows.next().map_err(database_error)? {
        let read = |i| row.get::<_, String>(i).map_err(database_error);
        let restore = || -> Result<Receivable, StorageError> {
            Ok(Receivable::new(
                read(0)?.parse()?,
                AccountName::new(&read(1)?)?,
                Note::new(&read(2)?)?,
                PositiveMoney::new(Money::from_minor(row.get(3).map_err(database_error)?)?)?,
                read(4)?.parse()?,
                CollectionTerms::new(
                    read(5)?.parse()?,
                    row.get(6).map_err(database_error)?,
                    row.get(7).map_err(database_error)?,
                )?,
            )?)
        };
        result.push(restore().map_err(|_| StorageError::Corrupt)?);
    }
    Ok(result)
}

fn database_error(error: rusqlite::Error) -> StorageError {
    match error.sqlite_error_code() {
        Some(rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase) => {
            StorageError::Corrupt
        }
        _ => StorageError::Unavailable,
    }
}

fn insert_settlement(
    connection: &Connection,
    recurring: RecurringId,
    month: Month,
    entry: EntryId,
) -> Result<(), StorageError> {
    connection.execute("INSERT INTO recurring_settlements(recurring_id,month,entry_id) VALUES(?1,?2,?3) ON CONFLICT(recurring_id,month) DO UPDATE SET entry_id=excluded.entry_id", params![recurring.to_string(), month.to_string(), entry.to_string()]).map_err(database_error)?;
    Ok(())
}

type StoredRecurring = (
    Vec<RecurringExpense>,
    Vec<ledger_application::RecurringSettlement>,
);
fn read_recurring(
    connection: &Connection,
    accounts: &[Account],
    entries: &[JournalEntry],
) -> Result<StoredRecurring, StorageError> {
    let mut query = connection.prepare("SELECT id,name,amount_minor,category,account_id,day,start_month,stopped_from,installments FROM recurring_expenses ORDER BY id").map_err(database_error)?;
    let mut rows = query.query([]).map_err(database_error)?;
    let mut schedules = Vec::new();
    while let Some(row) = rows.next().map_err(database_error)? {
        let read = |i| row.get::<_, String>(i).map_err(database_error);
        let restore = || -> Result<RecurringExpense, StorageError> {
            let account: Option<AccountId> = row
                .get::<_, Option<String>>(4)
                .map_err(database_error)?
                .map(|s| s.parse())
                .transpose()?;
            if account.is_some_and(|id| !accounts.iter().any(|a| a.id() == id)) {
                return Err(StorageError::Corrupt);
            }
            let mut schedule = RecurringExpense::new(
                read(0)?.parse()?,
                AccountName::new(&read(1)?)?,
                PositiveMoney::new(Money::from_minor(row.get(2).map_err(database_error)?)?)?,
                Category::from_code(&read(3)?)?,
                account,
                MonthlyDue::new(row.get(5).map_err(database_error)?, read(6)?.parse()?)?
                    .with_installments(row.get(8).map_err(database_error)?)?,
            )?;
            if let Some(end) = row.get::<_, Option<String>>(7).map_err(database_error)? {
                schedule = schedule.stop_from(end.parse()?)?;
            }
            Ok(schedule)
        };
        schedules.push(restore().map_err(|_| StorageError::Corrupt)?);
    }
    if schedules.len() > ledger_application::MAX_RECURRING {
        return Err(StorageError::Corrupt);
    }
    let mut query = connection.prepare("SELECT recurring_id,month,entry_id FROM recurring_settlements ORDER BY recurring_id,month").map_err(database_error)?;
    let mut rows = query.query([]).map_err(database_error)?;
    let mut settlements = Vec::new();
    while let Some(row) = rows.next().map_err(database_error)? {
        let parse = || -> Result<ledger_application::RecurringSettlement, StorageError> {
            Ok(ledger_application::RecurringSettlement {
                recurring: row.get::<_, String>(0).map_err(database_error)?.parse()?,
                month: row.get::<_, String>(1).map_err(database_error)?.parse()?,
                entry: row.get::<_, String>(2).map_err(database_error)?.parse()?,
            })
        };
        let settlement = parse().map_err(|_| StorageError::Corrupt)?;
        if !schedules.iter().any(|s| {
            s.id() == settlement.recurring && s.due().number_in(settlement.month).is_some()
        }) || !entries
            .iter()
            .any(|e| e.id() == settlement.entry && matches!(e.kind(), EntryKind::Expense { .. }))
        {
            return Err(StorageError::Corrupt);
        }
        settlements.push(settlement);
    }
    Ok((schedules, settlements))
}
