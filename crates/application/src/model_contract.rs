//! Boundary for an untrusted local model. Validation grants no write access.
use crate::AppError;
use serde::Deserialize;

const FIELDS: [&str; 8] = [
    "intent",
    "amount_text",
    "currency_text",
    "account_text",
    "destination_account_text",
    "category_text",
    "date_text",
    "description_text",
];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Proposal,
    Unsupported,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposedIntent {
    Expense,
    Income,
    Transfer,
    Summary,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub intent: ProposedIntent,
    pub amount_text: Option<String>,
    pub currency_text: Option<String>,
    pub account_text: Option<String>,
    pub destination_account_text: Option<String>,
    pub category_text: Option<String>,
    pub date_text: Option<String>,
    pub description_text: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidatedProposal {
    pub schema_version: String,
    pub status: ProposalStatus,
    pub candidates: Vec<Candidate>,
}

pub fn validate_proposal(source: &str, output: &str) -> Result<ValidatedProposal, AppError> {
    let invalid = || AppError::Input("ผลตีความไม่ผ่านการตรวจสอบ กรุณาใช้ตัวอย่างคำสั่งหรือแบบฟอร์ม".into());
    if source.chars().count() > 1000 || output.len() > 16_384 {
        return Err(invalid());
    }
    let shape: serde_json::Value = serde_json::from_str(output).map_err(|_| invalid())?;
    let candidates = shape
        .get("candidates")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(invalid)?;
    for candidate in candidates {
        let object = candidate.as_object().ok_or_else(invalid)?;
        if FIELDS.iter().any(|field| !object.contains_key(*field)) {
            return Err(invalid());
        }
    }
    let proposal: ValidatedProposal = serde_json::from_str(output).map_err(|_| invalid())?;
    if proposal.schema_version != "1" {
        return Err(invalid());
    }
    match proposal.status {
        ProposalStatus::Unsupported if !proposal.candidates.is_empty() => return Err(invalid()),
        ProposalStatus::Proposal if !(1..=3).contains(&proposal.candidates.len()) => {
            return Err(invalid());
        }
        _ => {}
    }
    for candidate in &proposal.candidates {
        for field in [
            &candidate.amount_text,
            &candidate.currency_text,
            &candidate.account_text,
            &candidate.destination_account_text,
            &candidate.category_text,
            &candidate.date_text,
            &candidate.description_text,
        ]
        .into_iter()
        .flatten()
        {
            if field.is_empty() || field.chars().count() > 160 || !source.contains(field) {
                return Err(invalid());
            }
        }
        match candidate.intent {
            ProposedIntent::Expense | ProposedIntent::Income
                if candidate.destination_account_text.is_some() =>
            {
                return Err(invalid());
            }
            ProposedIntent::Transfer
                if candidate.category_text.is_some()
                    || (candidate.account_text.is_some()
                        && candidate.account_text == candidate.destination_account_text) =>
            {
                return Err(invalid());
            }
            ProposedIntent::Summary
                if candidate.amount_text.is_some()
                    || candidate.currency_text.is_some()
                    || candidate.account_text.is_some()
                    || candidate.destination_account_text.is_some()
                    || candidate.category_text.is_some()
                    || candidate.description_text.is_some() =>
            {
                return Err(invalid());
            }
            _ => {}
        }
    }
    Ok(proposal)
}

/// A model can correct a misspelling or invent a category. Such labels never
/// become IDs: discard them and require manual selection. Money, currency,
/// dates, structure and authority fields still fail closed.
pub(crate) fn proposal_for_review(
    source: &str,
    output: &str,
) -> Result<(ValidatedProposal, bool), AppError> {
    if output.len() > 16_384 || source.chars().count() > 1000 {
        return validate_proposal(source, output).map(|proposal| (proposal, false));
    }
    let mut value: serde_json::Value = serde_json::from_str(output)
        .map_err(|_| AppError::Input("AI อ่านรายการไม่สำเร็จ กรุณาลองใหม่หรือใช้แบบฟอร์ม".into()))?;
    let mut discarded = false;
    if let Some(candidates) = value
        .get_mut("candidates")
        .and_then(serde_json::Value::as_array_mut)
    {
        for candidate in candidates {
            for field in [
                "account_text",
                "destination_account_text",
                "category_text",
                "description_text",
            ] {
                if let Some(value) = candidate.get_mut(field)
                    && value.as_str().is_some_and(|text| !source.contains(text))
                {
                    *value = serde_json::Value::Null;
                    discarded = true;
                }
            }
        }
    }
    validate_proposal(source, &value.to_string()).map(|proposal| (proposal, discarded))
}
