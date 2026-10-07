# Credit cards and financial reports

Set a statement closing day and payment due day from 1–31. Purchases on the closing day belong to that cycle; the following day starts the next. A due day after the closing day is in the same month, otherwise the following month. Short months clamp to their last day, including leap years.

Card purchases increase debt without reducing cash. A bank-to-card transfer reduces both the bank balance and card debt; it is not another expense. Payments allocate to the oldest charges first. Overpayment becomes a positive card balance. Cancellation uses a reversal rather than erasing the journal.

Recurring payments offer two explicit modes. **Repay existing credit card debt** selects a funding account and a destination card, reduces both balances, and marks the installment paid without another expense. Card-backed plans default to this mode; the funding account must be chosen. **New expense / new card purchase** records a new charge and increases debt when the selected account is a card. That charge still needs a later card repayment. Payment breakdowns distinguish the underlying charges from the repayment itself.

Existing card transfers can be linked to an installment without moving money again. Cancelling a repayment restores both balances and reopens the installment. New installment repayments cannot exceed the recorded debt, which is rechecked when saving.

## Overview amounts

- Assets include positive account balances, valued crypto holdings and outstanding receivable principal.
- Liabilities are negative account balances shown as debt.
- Net assets equal assets minus liabilities.
- The paid-out indicator includes cash-account expenses and non-card-to-card payments for its period. It excludes lending principal and ordinary asset transfers.
- Unrecorded recurring plans are obligations, not journal entries. Including them in a projection does not change account balances.

These indicators overlap; do not sum every displayed amount or subtract paid-out cash again from net assets. Principal collected from a debtor is an asset transfer; only interest is income.

Legacy cards without statement terms require those terms to be supplied. The app preserves existing history and does not guess a due cycle for opening debt with no cycle evidence. It does not connect to banks or calculate bank interest, penalties, minimum payments, holiday adjustments or bank-specific installment rules.
