# Quick entry and review

Quick entry has a composer and a manual-entry tab. The composer toolbar provides receipt attachment, supported Android voice input, a prompt-examples modal and the current model selector. Models can be selected or downloaded here and removed in Settings.

Use the in-app examples for the supported Thai command grammar. English interface text does not change that grammar into an English parser. Supported actions cover income, expenses, transfers/card principal payments, account creation/deletion, recurring-plan creation/payment/linking/term count/stopping, opening receivables, new loans, principal/interest collection and cancellation by reversal.

## Review before writing

1. Enter text or choose a prompt example and replace its sample values.
2. Read the entries. The app opens editable modal cards, one for each proposed action.
3. Supply missing accounts, categories, dates or debt details. Ambiguous names require a selection from existing records.
4. Review all actions and confirm. Previewing, parsing, speech input and model inference do not write the ledger.

The deterministic parser consumes the entire input. It does not discard unknown trailing text or assume the first number is the amount. Quoted names support spaces and words that would otherwise be grammar keywords. Money supports at most two decimal places. Dates use Gregorian years; relative dates freeze when submitted.

One text batch accepts at most 8 actions and 1,000 characters. Newlines and semicolons separate commands; the in-app examples also demonstrate supported Thai conjunctions. Later actions may refer to an account or receivable created earlier in the same batch. Confirmation is atomic: failure in any action leaves no partial batch. A repeated submission ID cannot duplicate the same batch; changed underlying data requires a fresh review.

Free text outside the grammar may use an optional [local model](local-model-selection.md) for a bounded single-entry proposal. Unsupported or malformed multi-entry text is not reduced to only its first entry. There is no silent fuzzy account matching or model-generated SQL. Schema, source text, amounts, dates, names and policy are checked in Rust before confirmation. Valid JSON alone does not establish financial correctness.

Receipt details remain subject to [exact amount reconciliation](receipt-ocr.md). Speech only fills draft text. Unsaved drafts and receipt images are not part of a backup.
