# Implemented Thai tax worksheet

This is the coverage of rule pack `TH-PIT-2568-2569-core-2026-09-24` in the application, not a claim that every Thai tax case is supported. It covers confirmed annual domestic income for Thai-resident individuals filing separately for 2025 and 2026 (B.E. 2568 and 2569). Gross income is not inferred from a net bank deposit.

## Implemented rules

| Item | Implemented treatment |
| --- | --- |
| Sections 40(1) and 40(2) | Shared 50% expense deduction capped at THB 100,000 |
| Eligible section 40(3) royalties and 40(5)–(8) | User-confirmed qualifying actual expenses; no automatic flat-rate classification |
| Section 40(4) interest elected for inclusion | No expense deduction; no dividend credit or automatic final-tax election |
| Personal and eligible non-earning spouse allowance | THB 60,000 each |
| Eligible children | THB 30,000 each, plus THB 30,000 for qualifying second/subsequent legal children born from 2018; eligibility is user-confirmed |
| Eligible parents and supported disabled dependants | THB 30,000 and 60,000 per qualifying person respectively |
| Own life/health insurance | Health cap THB 25,000; combined cap THB 100,000 |
| Parents' health insurance and home interest | Qualifying personal share capped at THB 15,000 and 100,000 respectively |
| Section 33 social security | Actual contribution capped at THB 9,000 for 2025 and 10,500 for 2026 |
| Ordinary single-rate donations | Capped at 10% after expenses and allowances; eligibility/evidence is user-confirmed |
| Progressive calculation | 0/5/10/15/20/25/30/35% brackets; first THB 150,000 of net income exempt |
| Section 48(2) alternative minimum | Included 40(2)–(8) gross income from THB 120,000, at 0.5%; exempt under this method if no more than THB 5,000, otherwise compare with 48(1) |
| Paid credits | Withholding on included income plus interim/prepaid tax; extra payable/refundable result, with sub-baht truncation of extra payable after credits |

Input changes invalidate the previous result. Unsupported-case selections block the final summary rather than treating unknown liabilities or relief as zero. Rights and supporting evidence must be confirmed by the user; neither a category name nor AI establishes eligibility.

## Explicitly unsupported

Retirement funds/RMF, pension insurance, Thai ESG/ESGX, special annual measures, sections 39/40 social security, spouse life insurance, double-rate donations, elderly/disability exemptions, foreign income/foreign tax credit, dividend credits, lump-sum treatments, flat-rate expense elections, special rates and joint spouse filing are outside this pack.

The worksheet is not a filing service or an expert-certified return. Its temporary form survives navigation but not app closure. It has not been differentially verified against the Revenue Department's e-Filing implementation. No foreign-service VAT remittance or general VAT/withholding accounting workflow is implemented.

Income metadata can reconcile gross, VAT, withholding, other deductions and net cash; see [taxable income entries](tax-income-entries.md). The journal currently uses balanced two-posting entries. Metadata or receipt notes must not be presented as a VAT subledger, withholding liability or completed tax filing.

## Implementation and provenance

The engine and its tests are in `crates/application/src/tax.rs` and `crates/application/tests/tax.rs`. Calculations use fixed-point amounts and explicit caps/rounding, not language-model arithmetic.

The rule pack's recorded source review was on 2026-09-24. Its references include the Revenue Department's [rates](https://www.rd.go.th/59670.html), [calculation methods](https://www.rd.go.th/555.html), [expense rules](https://www.rd.go.th/556.html), [Revenue Code](https://www.rd.go.th/5937.html), [health insurance](https://www.rd.go.th/62777.html), [2025 PND90 instructions](https://www.rd.go.th/fileadmin/tax_pdf/pit/2568/Ins90_241268.pdf), [payment rounding](https://efiling.rd.go.th/rd-cms/bank) and the [published social-security wage ceiling](https://ratchakitcha.soc.go.th/documents/98728.pdf). Updating this English guide does not revalidate those rules for a new tax year; new years or legal changes require a separately reviewed rule pack and boundary tests.
