# Version 0.1.14

Transactions now use the transaction date, newest first. Entries on the same day retain their latest-recorded order. Sorting and filters run before pagination and never alter the underlying journal sequence.

Use **From date** and **Through date** for an inclusive range; use the same date in both fields to show one day. Either endpoint can be empty. Choosing a date switches the year selector to **All years**, so ranges can cross December and January. Choosing a year clears the date range. Type tabs combine with both controls, and changing a filter returns to the first page. An inverted range displays a validation message.

The account dialog now handles native close events, including Android Back/Escape, so it can be opened again after dismissal. Its close controls remain available while work is pending. Closing a submitted form does not cancel the underlying save; the app refreshes and shows the result when it finishes. Duplicate submits remain blocked while a save is pending. Validation errors remain in the form so the values can be corrected and submitted again.

Monthly bill summaries display the progress description below the ring. On small screens, category tabs wrap into two columns, month navigation uses a fixed row below its label, and amounts can wrap without overlapping.

The ledger schema and installed data location are unchanged. Windows MSI/ZIP and Android ARM64/x86_64 test APKs are included. Windows packages remain unsigned, and Android retains its existing test application ID and signing identity.
