//! Annual Thai PIT worksheet. Rules checked 2026-09-24; this is not a filing API.
//! Eligibility and actual expenses require evidence. No ledger deposit is treated
//! as taxable gross. Unsupported elections/exemptions must not yield a final result.
use crate::AppError;
use ledger_domain::Money;

pub const TAX_RULE_VERSION: &str = "TH-PIT-2568-2569-core-2026-09-24";
pub const TAX_SOURCES: [(&str, &str); 6] = [
    (
        "เพดานประกันสังคม ม.33 ตั้งแต่ 2569",
        "https://ratchakitcha.soc.go.th/documents/98728.pdf",
    ),
    ("อัตราก้าวหน้า", "https://www.rd.go.th/59670.html"),
    ("วิธีคำนวณและภาษีขั้นต่ำ 0.5%", "https://www.rd.go.th/555.html"),
    ("ค่าใช้จ่ายแยกตามประเภทเงินได้", "https://www.rd.go.th/556.html"),
    ("ประมวลรัษฎากร ม.40–48", "https://www.rd.go.th/5937.html"),
    ("ประกันชีวิตและสุขภาพ", "https://www.rd.go.th/62777.html"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxWorksheet {
    pub year: u16,
    /// Assessable domestic income before withholding, by section 40(1)..40(8).
    pub incomes: [String; 8],
    /// Documented eligible actual expenses, used only for sections 40(3),(5)..(8).
    pub expenses: [String; 8],
    pub life_insurance: String,
    pub health_insurance: String,
    pub parent_health: String,
    pub mortgage_interest: String,
    pub social_security: String,
    pub donation: String,
    pub withholding: String,
    pub prepayments: String,
    pub spouse_no_income: bool,
    pub eligible_children: u16,
    pub additional_children: u16,
    pub eligible_parents: u16,
    pub eligible_disabled: u16,
    pub eligibility_confirmed: bool,
    pub unsupported_items: bool,
}
impl Default for TaxWorksheet {
    fn default() -> Self {
        Self {
            year: 2569,
            incomes: std::array::from_fn(|_| "0".into()),
            expenses: std::array::from_fn(|_| "0".into()),
            life_insurance: "0".into(),
            health_insurance: "0".into(),
            parent_health: "0".into(),
            mortgage_interest: "0".into(),
            social_security: "0".into(),
            donation: "0".into(),
            withholding: "0".into(),
            prepayments: "0".into(),
            spouse_no_income: false,
            eligible_children: 0,
            additional_children: 0,
            eligible_parents: 0,
            eligible_disabled: 0,
            eligibility_confirmed: false,
            unsupported_items: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxBand {
    pub lower_baht: i64,
    pub upper_baht: Option<i64>,
    pub rate_percent: u8,
    pub taxable: Money,
    pub tax: Money,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxResult {
    pub year: u16,
    pub gross: Money,
    pub expenses: Money,
    pub allowances: Vec<(&'static str, Money)>,
    pub donation_allowed: Money,
    pub net_taxable: Money,
    pub progressive: Money,
    pub minimum_raw: Money,
    pub minimum_applies: bool,
    pub tax_before_credits: Money,
    pub credits: Money,
    pub payable: Money,
    pub overpaid: Money,
    pub bands: Vec<TaxBand>,
    pub warnings: Vec<String>,
}

// Sub-satang precision keeps half-satang expense deductions, percentage caps,
// bracket taxes and the 5,000-baht exemption comparison exact. Truncate only
// displayed satang; payment discards fractional baht AFTER subtracting credits.
const SCALE: i128 = 1_000_000;
fn baht(value: i64) -> i128 {
    i128::from(value) * 100 * SCALE
}
fn read(text: &str) -> Result<i128, AppError> {
    let money: Money = text.parse().map_err(|_| {
        AppError::Input("กรอกจำนวนเงินบาทให้ครบ ใช้ตัวเลขไม่ติดลบและทศนิยมไม่เกิน 2 ตำแหน่ง".into())
    })?;
    if money.minor() < 0 {
        return Err(AppError::Input("จำนวนเงินภาษีต้องไม่ติดลบ".into()));
    }
    Ok(i128::from(money.minor()) * SCALE)
}
fn money(value: i128) -> Result<Money, AppError> {
    Ok(Money::from_minor(i64::try_from(value / SCALE).map_err(
        |_| AppError::Input("ยอดภาษีเกินขอบเขตที่รองรับ".into()),
    )?)?)
}

pub fn calculate_tax(input: &TaxWorksheet) -> Result<TaxResult, AppError> {
    if ![2568, 2569].contains(&input.year) {
        return Err(AppError::Input("ชุดกฎนี้รองรับเฉพาะปีภาษี 2568–2569".into()));
    }
    if input.unsupported_items {
        return Err(AppError::Input("กรณีนี้มีสิทธิหรือเงินได้ที่ชุดกฎยังไม่ครอบคลุม จึงยังสรุปภาษีไม่ได้ กรุณาตรวจตามแบบกรมสรรพากร รวมสิทธิให้ครบก่อนใช้ยอดเพื่อยื่น".into()));
    }
    if !input.eligibility_confirmed {
        return Err(AppError::Input(
            "ยืนยันประเภทเงินได้ ค่าใช้จ่าย และเงื่อนไขสิทธิตามหลักฐานก่อนคำนวณ".into(),
        ));
    }
    if input.additional_children > input.eligible_children
        || input.eligible_parents > 4
        || (!input.spouse_no_income && input.eligible_parents > 2)
    {
        return Err(AppError::Input(
            "ตรวจจำนวนบุตรที่ใช้สิทธิเพิ่มเติมและบิดามารดาที่มีสิทธิ โดยไม่ใช้สิทธิซ้ำกับผู้อื่น".into(),
        ));
    }
    let incomes = input
        .incomes
        .iter()
        .map(|v| read(v))
        .collect::<Result<Vec<_>, _>>()?;
    let actual = input
        .expenses
        .iter()
        .map(|v| read(v))
        .collect::<Result<Vec<_>, _>>()?;
    let gross: i128 = incomes.iter().sum();
    let mut expense = ((incomes[0] + incomes[1]) / 2).min(baht(100_000));
    if actual[0] != 0 || actual[1] != 0 || actual[3] != 0 {
        return Err(AppError::Input(
            "ม.40(1),(2) หักเหมา 50% รวมไม่เกิน 100,000 บาท และ ม.40(4) หักค่าใช้จ่ายไม่ได้".into(),
        ));
    }
    for index in [2, 4, 5, 6, 7] {
        if actual[index] > incomes[index] {
            return Err(AppError::Input(format!(
                "ค่าใช้จ่าย ม.40({}) มากกว่าเงินได้ กรุณาตรวจหลักฐาน",
                index + 1
            )));
        }
        expense += actual[index];
    }
    let mut warnings = Vec::new();
    if input.year == 2569 {
        warnings.push("ปี 2569 ยังไม่สิ้นปี ผลนี้เป็นประมาณการตามข้อมูลที่กรอกและกฎที่ตรวจ ณ 24 ก.ย. 2569 ต้องตรวจมาตรการและแบบสิ้นปีอีกครั้ง".into());
    }
    if [2, 4, 5, 6, 7].iter().any(|i| incomes[*i] > 0) {
        warnings.push("ม.40(3),(5)–(8) ใช้ค่าใช้จ่ายจริงที่ผู้ใช้ตรวจสิทธิแล้ว ไม่เลือกอัตราเหมาให้ เพราะขึ้นกับลักษณะเงินได้และกิจการ".into());
    }
    let mut allowances = vec![("ส่วนตัว", baht(60_000))];
    if input.spouse_no_income {
        allowances.push(("คู่สมรสไม่มีเงินได้", baht(60_000)));
    }
    allowances.push((
        "บุตรที่มีสิทธิ รวมสิทธิเพิ่มเติม",
        baht(30_000)
            * (i128::from(input.eligible_children) + i128::from(input.additional_children)),
    ));
    allowances.push((
        "อุปการะบิดามารดาที่มีสิทธิ",
        baht(30_000) * i128::from(input.eligible_parents),
    ));
    allowances.push((
        "อุปการะผู้พิการ/ทุพพลภาพที่มีสิทธิ",
        baht(60_000) * i128::from(input.eligible_disabled),
    ));
    let life = read(&input.life_insurance)?.min(baht(100_000));
    let health = read(&input.health_insurance)?
        .min(baht(25_000))
        .min(baht(100_000) - life);
    allowances.push(("ประกันชีวิตและสุขภาพตนเอง (เพดานร่วม)", life + health));
    if read(&input.life_insurance)? + read(&input.health_insurance)? > life + health {
        warnings.push(
            "ใช้ประกันสุขภาพได้ไม่เกิน 25,000 บาท และรวมประกันชีวิตไม่เกิน 100,000 บาท แสดงเฉพาะส่วนที่ใช้ได้"
                .into(),
        );
    }
    for (label, entered, cap) in [
        ("ประกันสุขภาพบิดามารดา", &input.parent_health, 15_000),
        ("ดอกเบี้ยที่อยู่อาศัย", &input.mortgage_interest, 100_000),
    ] {
        let paid = read(entered)?;
        if paid > baht(cap) {
            warnings.push(format!("{label}ใช้ได้ไม่เกิน {cap} บาทตามสิทธิ"));
        }
        allowances.push((label, paid.min(baht(cap))));
    }
    let social = read(&input.social_security)?;
    let social_cap = baht(if input.year == 2569 { 10_500 } else { 9_000 });
    if social > social_cap {
        warnings.push(
            "ประกันสังคม ม.33 เกินเพดานรายปี ใช้ได้ตามจ่ายจริงไม่เกินเพดาน และต้องตรวจเดือนที่ได้รับลดอัตราเป็นพิเศษ"
                .into(),
        );
    }
    allowances.push(("ประกันสังคม ม.33 ตามจ่ายจริง", social.min(social_cap)));
    let after_allowances =
        (gross - expense - allowances.iter().map(|(_, v)| v).sum::<i128>()).max(0);
    let donation = read(&input.donation)?.min(after_allowances / 10);
    if read(&input.donation)? > donation {
        warnings.push("เงินบริจาคทั่วไปใช้ได้ไม่เกิน 10% ของเงินได้หลังหักค่าใช้จ่ายและค่าลดหย่อน".into());
    }
    let net = after_allowances - donation;
    let mut bands = Vec::new();
    let mut progressive = 0;
    let mut lower = 0;
    for (upper, rate) in [
        (Some(150_000), 0),
        (Some(300_000), 5),
        (Some(500_000), 10),
        (Some(750_000), 15),
        (Some(1_000_000), 20),
        (Some(2_000_000), 25),
        (Some(5_000_000), 30),
        (None, 35),
    ] {
        let taxable = (net.min(upper.map(baht).unwrap_or(net)) - baht(lower)).max(0);
        let tax = taxable * i128::from(rate) / 100;
        progressive += tax;
        bands.push(TaxBand {
            lower_baht: lower,
            upper_baht: upper,
            rate_percent: rate,
            taxable: money(taxable)?,
            tax: money(tax)?,
        });
        if let Some(upper) = upper {
            lower = upper;
        }
    }
    let non_salary: i128 = incomes[1..].iter().sum();
    let minimum = if non_salary >= baht(120_000) {
        non_salary * 5 / 1000
    } else {
        0
    };
    let minimum_applies = minimum > baht(5_000);
    let tax = progressive.max(if minimum_applies { minimum } else { 0 });
    let credits = read(&input.withholding)? + read(&input.prepayments)?;
    let due = (tax - credits).max(0);
    let payable = due / baht(1) * baht(1);
    Ok(TaxResult {
        year: input.year,
        gross: money(gross)?,
        expenses: money(expense)?,
        allowances: allowances
            .into_iter()
            .map(|(label, value)| Ok((label, money(value)?)))
            .collect::<Result<_, AppError>>()?,
        donation_allowed: money(donation)?,
        net_taxable: money(net)?,
        progressive: money(progressive)?,
        minimum_raw: money(minimum)?,
        minimum_applies,
        tax_before_credits: money(tax)?,
        credits: money(credits)?,
        payable: money(payable)?,
        overpaid: money((credits - tax).max(0))?,
        bands,
        warnings,
    })
}
