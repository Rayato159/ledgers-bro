#![allow(clippy::expect_used)]
use ledger_application::*;
use ledger_domain::Money;
fn worksheet() -> TaxWorksheet {
    TaxWorksheet {
        eligibility_confirmed: true,
        ..Default::default()
    }
}
fn m(s: &str) -> Money {
    s.parse().expect("money")
}
#[test]
fn salary_600000_gives_21500_before_credits() {
    let mut input = worksheet();
    input.incomes[0] = "600000".into();
    let result = calculate_tax(&input).expect("tax");
    assert_eq!(result.expenses, m("100000"));
    assert_eq!(result.net_taxable, m("440000"));
    assert_eq!(result.progressive, m("21500"));
    assert_eq!(result.minimum_raw, Money::ZERO);
    assert_eq!(result.payable, m("21500"));
}
#[test]
fn every_progressive_boundary_and_first_baht_above_it() {
    // Salary + 160,000 gives this net after the expense and personal allowances.
    for (net, tax, next_rate) in [
        (150000, 0, 5),
        (300000, 7500, 10),
        (500000, 27500, 15),
        (750000, 65000, 20),
        (1000000, 115000, 25),
        (2000000, 365000, 30),
        (5000000, 1265000, 35),
    ] {
        let mut input = worksheet();
        input.incomes[0] = (net + 160000).to_string();
        assert_eq!(
            calculate_tax(&input).expect("boundary").progressive,
            m(&tax.to_string())
        );
        input.incomes[0] = (net + 160001).to_string();
        assert_eq!(
            calculate_tax(&input).expect("above").progressive.minor(),
            tax * 100 + next_rate
        );
    }
}
#[test]
fn salary_and_services_share_one_expense_cap() {
    let mut input = worksheet();
    input.incomes[0] = "180000".into();
    input.incomes[1] = "120000".into();
    let result = calculate_tax(&input).expect("mixed");
    assert_eq!(result.expenses, m("100000"));
    assert_eq!(result.minimum_raw, m("600"));
    assert!(!result.minimum_applies);
    assert_eq!(result.payable, Money::ZERO);
    input.incomes[0] = "90000.01".into();
    input.incomes[1] = "10000".into();
    assert_eq!(
        calculate_tax(&input).expect("fraction").expenses,
        m("50000")
    );
}
#[test]
fn minimum_tax_exemption_is_not_a_5000_deduction_and_excludes_salary() {
    for (income, minimum, applies, payable) in [
        ("119999.99", "0", false, "0"),
        ("120000", "600", false, "0"),
        ("1000000", "5000", false, "0"),
        ("1000000.01", "5000", true, "5000"),
        ("1200000", "6000", true, "6000"),
    ] {
        let mut input = worksheet();
        input.incomes[7] = income.into();
        input.expenses[7] = income.into();
        let result = calculate_tax(&input).expect("business actual expenses");
        assert_eq!(result.minimum_raw, m(minimum));
        assert_eq!(result.minimum_applies, applies);
        assert_eq!(result.payable, m(payable));
    }
    let mut input = worksheet();
    input.incomes[0] = "2000000".into();
    input.incomes[7] = "1200000".into();
    input.expenses[7] = "1200000".into();
    let result = calculate_tax(&input).expect("larger progressive");
    assert_eq!(result.minimum_raw, m("6000"));
    assert_eq!(result.tax_before_credits, result.progressive);
}
#[test]
fn caps_joint_insurance_donation_and_social_security_by_year() {
    let mut input = worksheet();
    input.incomes[0] = "1000000".into();
    input.life_insurance = "90000".into();
    input.health_insurance = "50000".into();
    input.parent_health = "20000".into();
    input.mortgage_interest = "120000".into();
    input.social_security = "20000".into();
    input.donation = "200000".into();
    let result = calculate_tax(&input).expect("caps");
    assert_eq!(result.donation_allowed, m("61450"));
    assert_eq!(result.net_taxable, m("553050"));
    assert_eq!(
        result
            .allowances
            .iter()
            .find(|(name, _)| name.contains("ประกันชีวิต"))
            .expect("insurance")
            .1,
        m("100000")
    );
    input.year = 2568;
    assert_eq!(
        calculate_tax(&input).expect("2568").donation_allowed,
        m("61600")
    );
    input.life_insurance = "0".into();
    input.donation = "0".into();
    assert_eq!(
        calculate_tax(&input)
            .expect("health cap")
            .allowances
            .iter()
            .find(|(name, _)| name.contains("ประกันชีวิต"))
            .expect("insurance")
            .1,
        m("25000")
    );
}
#[test]
fn withholding_and_prepayments_are_subtracted_before_payment_fraction_is_dropped() {
    let mut input = worksheet();
    input.incomes[0] = "600009.99".into();
    input.withholding = "10000.60".into();
    input.prepayments = "10000".into();
    let result = calculate_tax(&input).expect("credits");
    assert_eq!(result.progressive, m("21500.99"));
    assert_eq!(result.payable, m("1500"));
    input.withholding = "30000".into();
    let result = calculate_tax(&input).expect("overpayment");
    assert_eq!(result.payable, Money::ZERO);
    assert_eq!(result.overpaid, m("18499"));
}
#[test]
fn entitled_family_allowances_and_zero_floor() {
    let mut input = worksheet();
    input.incomes[0] = "1000000".into();
    input.spouse_no_income = true;
    input.eligible_children = 2;
    input.additional_children = 1;
    input.eligible_parents = 4;
    input.eligible_disabled = 1;
    assert_eq!(
        calculate_tax(&input).expect("family").net_taxable,
        m("510000")
    );
    input.incomes[0] = "0".into();
    assert_eq!(calculate_tax(&input).expect("zero").payable, Money::ZERO);
    input.eligible_children = u16::MAX;
    input.additional_children = u16::MAX;
    assert_eq!(
        calculate_tax(&input)
            .expect("no count overflow")
            .net_taxable,
        Money::ZERO
    );
}
#[test]
fn incomplete_unsupported_and_invalid_inputs_cannot_show_a_final_tax() {
    assert!(calculate_tax(&TaxWorksheet::default()).is_err());
    let mut input = worksheet();
    input.unsupported_items = true;
    assert!(calculate_tax(&input).is_err());
    input = worksheet();
    input.year = 2567;
    assert!(calculate_tax(&input).is_err());
    for amount in ["", "-1", "abc", "1.001", "90000000001"] {
        input = worksheet();
        input.incomes[0] = amount.into();
        assert!(calculate_tax(&input).is_err());
    }
    input = worksheet();
    input.expenses[3] = "1".into();
    assert!(calculate_tax(&input).is_err());
    input = worksheet();
    input.expenses[7] = "1".into();
    assert!(calculate_tax(&input).is_err());
    input = worksheet();
    input.eligible_parents = 3;
    assert!(calculate_tax(&input).is_err());
    input = worksheet();
    input.additional_children = 1;
    assert!(calculate_tax(&input).is_err());
}
