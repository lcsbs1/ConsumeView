//! Cost calculation from pricing.json rates (per 1M tokens).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelRates {
    pub input: f64,
    pub output: f64,
    pub cached: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PricingTable {
    pub models: HashMap<String, ModelRates>,
}

impl PricingTable {
    pub fn rates_for(&self, model: &str) -> Option<&ModelRates> {
        if let Some(r) = self.models.get(model) {
            return Some(r);
        }
        // Prefix / family fallback: longest matching key prefix
        let mut best: Option<(&String, &ModelRates)> = None;
        for (key, rates) in &self.models {
            if model.starts_with(key) {
                match best {
                    None => best = Some((key, rates)),
                    Some((prev, _)) if key.len() > prev.len() => best = Some((key, rates)),
                    _ => {}
                }
            }
        }
        best.map(|(_, r)| r)
    }
}

pub fn load_pricing(json: &str) -> Result<PricingTable, serde_json::Error> {
    serde_json::from_str(json)
}

/// USD cost from token counts using per-1M rates.
/// Cached tokens are billed at the cached rate and subtracted from prompt for input billing
/// when `cached_tokens` is provided (OpenAI-style prompt includes cached).
pub fn estimate_cost_usd(
    rates: &ModelRates,
    prompt_tokens: i64,
    completion_tokens: i64,
    cached_tokens: i64,
) -> f64 {
    let cached = cached_tokens.max(0) as f64;
    let prompt = prompt_tokens.max(0) as f64;
    let completion = completion_tokens.max(0) as f64;
    let billable_input = (prompt - cached).max(0.0);
    (billable_input * rates.input + completion * rates.output + cached * rates.cached) / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn costs_from_seed() {
        let json = include_str!("../../pricing.json");
        let table = load_pricing(json).unwrap();
        let rates = table.rates_for("gemini-2.0-flash").unwrap();
        // 1M input @ 0.10 = 0.10
        let c = estimate_cost_usd(rates, 1_000_000, 0, 0);
        assert!((c - 0.10).abs() < 1e-9);
    }
}
