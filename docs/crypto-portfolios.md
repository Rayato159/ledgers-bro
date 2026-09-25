# BTC and SOL portfolios

Create a crypto account under **Accounts** and enter BTC and SOL quantities, using zero for assets not held. BTC supports 8 decimal places and SOL supports 9; quantities are stored as integer base units. Edit quantities from the portfolio card.

THB ledgers value holdings using public CoinGecko prices. Each asset's quantity times its THB price is rounded to minor currency units before totals are combined. Portfolio valuations contribute to assets and net assets; they are estimates rather than executable trade quotes.

- Prices refresh roughly every 60 seconds while the app is open and a portfolio holds coins. Manual refresh is rate-limited.
- Failures back off up to roughly 8 minutes and retain the last saved quote.
- Provider timestamps are displayed. Quotes older than 3 minutes are marked stale; timestamps more than 5 minutes ahead are rejected.
- Holdings with no available quote show an unavailable valuation and are excluded from the total with an explanation. Empty holdings can be valued at zero without a quote.

Requests send only public asset IDs (`bitcoin`, `solana`) and `thb`, not quantities, account names, wallet addresses or transactions. HTTPS requests have an 8-second timeout and run outside the ledger/UI work.

Legacy portfolios entered as a cash value keep their existing journal history. Enter actual coin quantities to switch the overview to coin valuation; the old and new values are not added together. First move any recurring plan that uses the old portfolio to a compatible cash/bank account.

Quantity adjustments have their own history and do not create cash income, expense, trade profit or tax basis. Coin portfolios cannot be selected for ordinary cash transactions. The app does not connect wallets, place trades or calculate crypto taxes. Accounting CSV reports remain based on journal postings, without automatic unrealized valuation income.
