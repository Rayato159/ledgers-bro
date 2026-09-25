# Original modern fantasy cast

Historical artwork from 0.1.2. The current application uses [Lookhin's uncle crab](../uncle-crab/README.md); the files here are retained as design history.

Created with Codex's built-in image generator for Ledgers Bro 0.1.2. The user's final direction was **new original kawaii anime characters**, followed by **Modern Fantasy outfits**. The user's black-haired original mascot and the earlier tanuki are not the identity reference for this cast.

The generated Lumi portrait establishes the common linework and fashion vocabulary: soft pencil/digital ink grain, gentle pastel cel shading, charcoal urban jackets, ivory high collars, asymmetric short capes, silver constellation details and compact footwear. Each illustration was generated separately on a transparent background and inspected before use.

| Asset | Character / role | App use |
| --- | --- | --- |
| `masters/lumi.png` | Lumi, lavender-haired ledger keeper, blue star clip | App icon, login, overview, settings |
| `masters/mint.png` | Mint, braided mint-haired treasury mage, wallet | Financial accounts, first account, receivables |
| `masters/peach.png` | Peach, pink ponytail receipt scribe, stylus and receipt | Transactions, manual entry |
| `masters/skye.png` | Skye, blue-haired timekeeper, calendar and watch | Monthly bills and debts |
| `masters/iris.png` | Iris, silver-haired tax archivist, calculator and folder | Tax |
| `masters/ren.png` | Ren, cream-haired guide with headphones and phone | Chat, voice and quick entry |

Masters are retained unchanged. `scripts/prepare-art-assets.py` only resizes, adds transparent safe-area padding for launcher masks, and converts file formats. It does not redraw or recolor artwork. UI cutouts preserve generated alpha and their original colors; backgrounds and controls follow the user's selected theme. Generation briefs are in [prompts.md](prompts.md).

This cast is newly generated original artwork, not a licensed anime franchise. The user's supplied originals are not redistributed in these assets.
