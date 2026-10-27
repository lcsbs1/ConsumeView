import { useEffect, useState } from "react";
import { api, formatTokens, formatUsd, OverviewMetrics } from "../lib/api";

type Period = "today" | "week" | "month";

type Props = {
  refreshKey: number;
};

export function OverviewPanel({ refreshKey }: Props) {
  const [period, setPeriod] = useState<Period>("today");
  const [data, setData] = useState<OverviewMetrics | null>(null);

  useEffect(() => {
    void api
      .getOverview(period)
      .then(setData)
      .catch(console.error);
  }, [period, refreshKey]);

  const gemini = data?.split.gemini_cost_usd ?? 0;
  const grok = data?.split.grok_cost_usd ?? 0;
  const total = gemini + grok;
  const geminiPct = total > 0 ? (gemini / total) * 100 : 50;
  const grokPct = total > 0 ? (grok / total) * 100 : 50;

  return (
    <section className="animate-rise space-y-6">
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h2 className="font-display text-2xl text-foam">Overview</h2>
          <p className="mt-1 text-sm text-ink-300">
            Period totals, provider split, and month-end cost projection.
          </p>
        </div>
        <div className="flex gap-1 rounded-lg bg-ink-900/70 p-1">
          {(["today", "week", "month"] as Period[]).map((p) => (
            <button
              key={p}
              type="button"
              onClick={() => setPeriod(p)}
              className={`rounded-md px-3 py-1.5 text-sm capitalize ${
                period === p ? "bg-ink-700 text-foam" : "text-ink-500 hover:text-ink-300"
              }`}
            >
              {p}
            </button>
          ))}
        </div>
      </header>

      <div className="grid grid-cols-2 gap-x-8 gap-y-5 border-y border-ink-700/80 py-5 md:grid-cols-4">
        <Stat label="Total cost" value={formatUsd(data?.cost_usd ?? 0)} accent />
        <Stat label="Tokens" value={formatTokens(data?.total_tokens ?? 0)} />
        <Stat label="Avg tok/s" value={(data?.avg_tok_per_s ?? 0).toFixed(1)} />
        <Stat label="Avg TTFT" value={`${(data?.avg_ttft_ms ?? 0).toFixed(0)} ms`} />
      </div>

      <div>
        <div className="mb-2 flex justify-between text-sm text-ink-300">
          <span>Gemini vs Grok (cost)</span>
          <span className="font-mono text-xs">
            {formatUsd(gemini)} / {formatUsd(grok)}
          </span>
        </div>
        <div className="relative flex h-3 overflow-hidden rounded-full bg-ink-800">
          <div
            className="bg-tide transition-all duration-500"
            style={{ width: `${geminiPct}%` }}
          />
          <div
            className="bg-ember transition-all duration-500"
            style={{ width: `${grokPct}%` }}
          />
          <div className="pointer-events-none absolute inset-0 overflow-hidden">
            <div className="h-full w-1/3 animate-sweep bg-gradient-to-r from-transparent via-white/10 to-transparent" />
          </div>
        </div>
        <div className="mt-2 flex gap-4 text-xs text-ink-500">
          <span className="inline-flex items-center gap-1.5">
            <span className="h-2 w-2 rounded-full bg-tide" /> Gemini
          </span>
          <span className="inline-flex items-center gap-1.5">
            <span className="h-2 w-2 rounded-full bg-ember" /> Grok
          </span>
        </div>
      </div>

      <div className="rounded-xl border border-dashed border-ink-700/80 bg-ink-900/40 px-5 py-4">
        <div className="text-xs uppercase tracking-[0.14em] text-ink-500">
          Projected month cost
        </div>
        <div className="mt-1 font-display text-3xl text-foam">
          {formatUsd(data?.projected_month_cost_usd ?? 0)}
        </div>
        <p className="mt-1 text-sm text-ink-500">
          From month-to-date daily average × days in month.
        </p>
      </div>
    </section>
  );
}

function Stat({
  label,
  value,
  accent,
}: {
  label: string;
  value: string;
  accent?: boolean;
}) {
  return (
    <div>
      <div className="text-xs uppercase tracking-[0.14em] text-ink-500">{label}</div>
      <div className={`mt-1 font-mono text-xl ${accent ? "text-ember" : "text-foam"}`}>
        {value}
      </div>
    </div>
  );
}
