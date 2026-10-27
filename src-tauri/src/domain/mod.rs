//! Pure domain logic — no network, filesystem, keyring, or UI I/O.

mod aggregates;
mod pricing;
mod routing;

#[allow(unused_imports)]
pub use aggregates::{
    compute_overview, project_month_cost, session_totals, OverviewMetrics, Period, ProviderSplit,
    RequestRow, SessionTotals,
};
#[allow(unused_imports)]
pub use pricing::{estimate_cost_usd, load_pricing, ModelRates, PricingTable};
#[allow(unused_imports)]
pub use routing::{resolve_upstream, Provider, UpstreamTarget};
