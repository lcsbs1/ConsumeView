//! Session and period aggregates, TTFT, tok/s, month-end projection.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    Today,
    Week,
    Month,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionTotals {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cached_tokens: i64,
    pub total_tokens: i64,
    pub cost_usd: f64,
    pub request_count: i64,
    /// Cached / prompt when prompt > 0.
    pub cache_reuse_rate: f64,
    pub avg_ttft_ms: f64,
    pub avg_tok_per_s: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderSplit {
    pub gemini_cost_usd: f64,
    pub grok_cost_usd: f64,
    pub gemini_tokens: i64,
    pub grok_tokens: i64,
    pub gemini_requests: i64,
    pub grok_requests: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OverviewMetrics {
    pub period: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cached_tokens: i64,
    pub total_tokens: i64,
    pub cost_usd: f64,
    pub request_count: i64,
    pub avg_ttft_ms: f64,
    pub avg_tok_per_s: f64,
    pub split: ProviderSplit,
    pub projected_month_cost_usd: f64,
}

/// One persisted request row used for pure aggregation.
#[derive(Debug, Clone)]
pub struct RequestRow {
    pub provider: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cached_tokens: i64,
    pub total_tokens: i64,
    pub cost_usd: f64,
    pub duration_ms: i64,
    pub ttft_ms: i64,
}

pub fn session_totals(rows: &[RequestRow]) -> SessionTotals {
    if rows.is_empty() {
        return SessionTotals::default();
    }
    let mut t = SessionTotals::default();
    let mut ttft_sum = 0.0_f64;
    let mut ttft_n = 0_i64;
    let mut tok_s_sum = 0.0_f64;
    let mut tok_s_n = 0_i64;

    for r in rows {
        t.prompt_tokens += r.prompt_tokens;
        t.completion_tokens += r.completion_tokens;
        t.cached_tokens += r.cached_tokens;
        t.total_tokens += r.total_tokens;
        t.cost_usd += r.cost_usd;
        t.request_count += 1;
        if r.ttft_ms > 0 {
            ttft_sum += r.ttft_ms as f64;
            ttft_n += 1;
        }
        if r.duration_ms > 0 && r.completion_tokens > 0 {
            tok_s_sum += (r.completion_tokens as f64) / (r.duration_ms as f64 / 1000.0);
            tok_s_n += 1;
        }
    }
    t.cache_reuse_rate = if t.prompt_tokens > 0 {
        t.cached_tokens as f64 / t.prompt_tokens as f64
    } else {
        0.0
    };
    t.avg_ttft_ms = if ttft_n > 0 { ttft_sum / ttft_n as f64 } else { 0.0 };
    t.avg_tok_per_s = if tok_s_n > 0 {
        tok_s_sum / tok_s_n as f64
    } else {
        0.0
    };
    t
}

pub fn project_month_cost(month_to_date_cost: f64, day_of_month: u32, days_in_month: u32) -> f64 {
    if day_of_month == 0 || days_in_month == 0 {
        return 0.0;
    }
    let daily_avg = month_to_date_cost / day_of_month as f64;
    daily_avg * days_in_month as f64
}

pub fn compute_overview(
    rows: &[RequestRow],
    period: Period,
    day_of_month: u32,
    days_in_month: u32,
    month_to_date_cost: f64,
) -> OverviewMetrics {
    let totals = session_totals(rows);
    let mut split = ProviderSplit::default();
    for r in rows {
        match r.provider.as_str() {
            "gemini" => {
                split.gemini_cost_usd += r.cost_usd;
                split.gemini_tokens += r.total_tokens;
                split.gemini_requests += 1;
            }
            "grok" => {
                split.grok_cost_usd += r.cost_usd;
                split.grok_tokens += r.total_tokens;
                split.grok_requests += 1;
            }
            _ => {}
        }
    }

    let projected = match period {
        Period::Month => project_month_cost(month_to_date_cost, day_of_month, days_in_month),
        _ => project_month_cost(month_to_date_cost, day_of_month, days_in_month),
    };

    OverviewMetrics {
        period: match period {
            Period::Today => "today".into(),
            Period::Week => "week".into(),
            Period::Month => "month".into(),
        },
        prompt_tokens: totals.prompt_tokens,
        completion_tokens: totals.completion_tokens,
        cached_tokens: totals.cached_tokens,
        total_tokens: totals.total_tokens,
        cost_usd: totals.cost_usd,
        request_count: totals.request_count,
        avg_ttft_ms: totals.avg_ttft_ms,
        avg_tok_per_s: totals.avg_tok_per_s,
        split,
        projected_month_cost_usd: projected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_month_from_daily_avg() {
        // $3 over 10 days → $9 for 30-day month
        let p = project_month_cost(3.0, 10, 30);
        assert!((p - 9.0).abs() < 1e-9);
    }
}
