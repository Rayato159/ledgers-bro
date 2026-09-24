use crate::AppError;
use ledger_domain::Currency;

/// Currency names outside quoted user data must agree with the ledger. A matching
/// ISO code is removed for the existing amount grammar; numeric values never change.
pub fn normalize_prompt_currency(source: &str, currency: Currency) -> Result<String, AppError> {
    let mismatch = || {
        AppError::Input(format!(
            "This ledger uses {}. Enter amounts in {} only; no currency conversion is performed. Quote account names or notes containing currency codes.",
            currency.code(),
            currency.code()
        ))
    };
    let mut output = String::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut index = 0;
    while index < source.len() {
        let rest = &source[index..];
        let Some(ch) = rest.chars().next() else {
            break;
        };
        if ch == '"' && !escaped {
            quoted = !quoted;
        }
        if !quoted {
            let before = source[..index].chars().next_back();
            let boundary = before.is_none_or(|c| !c.is_ascii_alphabetic());
            let code = [
                "THB", "USD", "EUR", "GBP", "AUD", "CAD", "SGD", "CNY", "JPY", "KRW", "KWD", "BHD",
                "CHF", "HKD",
            ]
            .into_iter()
            .find(|code| {
                boundary
                    && rest
                        .get(..code.len())
                        .is_some_and(|s| s.eq_ignore_ascii_case(code))
                    && rest[code.len()..]
                        .chars()
                        .next()
                        .is_none_or(|c| !c.is_ascii_alphabetic())
            });
            if let Some(code) = code {
                if code != currency.code() {
                    return Err(mismatch());
                }
                output.push(' ');
                index += code.len();
                continue;
            }
            if (rest.starts_with("บาท") || ch == '฿') && currency != Currency::Thb {
                return Err(mismatch());
            }
            // A bare dollar/yuan sign is ambiguous. Ask for an ISO code instead.
            if ['$', '¥', '₩'].contains(&ch) {
                return Err(mismatch());
            }
            if ['€', '£'].contains(&ch) {
                if (ch == '€' && currency != Currency::Eur)
                    || (ch == '£' && currency != Currency::Gbp)
                {
                    return Err(mismatch());
                }
                output.push(' ');
                index += ch.len_utf8();
                continue;
            }
        }
        output.push(ch);
        escaped = ch == '\\' && !escaped;
        index += ch.len_utf8();
    }
    Ok(output)
}
