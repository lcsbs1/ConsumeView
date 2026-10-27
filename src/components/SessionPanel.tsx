import { FormEvent, useEffect, useState } from "react";
import {
  api,
  formatTokens,
  formatUsd,
  SessionInfo,
  SessionTotals,
  StoredRequest,
} from "../lib/api";

type Props = {
  refreshKey: number;
};

export function SessionPanel({ refreshKey }: Props) {
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [active, setActive] = useState<string>("");
  const [totals, setTotals] = useState<SessionTotals | null>(null);
  const [history, setHistory] = useState<StoredRequest[]>([]);
  const [newName, setNewName] = useState("");

  async function load(sessionId?: string) {
    const list = await api.listSessions();
    setSessions(list);
    const current = sessionId ?? (await api.getActiveSession());
    setActive(current);
    const [t, h] = await Promise.all([
      api.getSessionTotals(current),
      api.getSessionHistory(current, 80),
    ]);
    setTotals(t);
    setHistory(h);
  }

  useEffect(() => {
    void load().catch(console.error);
  }, [refreshKey]);

  async function onSelect(id: string) {
    await api.setActiveSession(id);
    await load(id);
  }

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    const s = await api.createSession(newName || "New session");
    setNewName("");
    await api.setActiveSession(s.id);
    await load(s.id);
  }

  return (
    <section className="animate-rise space-y-6">
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h2 className="font-display text-2xl text-foam">Session</h2>
          <p className="mt-1 text-sm text-ink-300">
            Explicit active session for Cursor traffic through the local proxy.
          </p>
        </div>
        <form onSubmit={onCreate} className="flex gap-2">
          <input
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="Session name"
            className="rounded-lg border border-ink-700 bg-ink-900/80 px-3 py-2 text-sm outline-none ring-tide/40 focus:ring-2"
          />
          <button
            type="submit"
            className="rounded-lg border border-ink-700 px-3 py-2 text-sm text-ink-100 hover:border-tide hover:text-tide"
          >
            Create
          </button>
        </form>
      </header>

      <div className="flex flex-wrap gap-2">
        {sessions.map((s) => (
          <button
            key={s.id}
            type="button"
            onClick={() => void onSelect(s.id)}
            className={`rounded-lg px-3 py-1.5 text-sm transition ${
              s.id === active
                ? "bg-tide text-ink-950"
                : "bg-ink-800/80 text-ink-300 hover:text-foam"
            }`}
          >
            {s.name}
          </button>
        ))}
      </div>

      <div className="grid grid-cols-2 gap-x-8 gap-y-4 border-y border-ink-700/80 py-5 md:grid-cols-4">
        <Metric label="Prompt in" value={formatTokens(totals?.prompt_tokens ?? 0)} />
        <Metric label="Completion out" value={formatTokens(totals?.completion_tokens ?? 0)} />
        <Metric label="Session cost" value={formatUsd(totals?.cost_usd ?? 0)} accent />
        <Metric
          label="Cache reuse"
          value={`${((totals?.cache_reuse_rate ?? 0) * 100).toFixed(1)}%`}
        />
      </div>

      <div className="overflow-hidden rounded-xl border border-ink-700/70">
        <table className="w-full text-left text-sm">
          <thead className="bg-ink-900/90 text-xs uppercase tracking-[0.12em] text-ink-500">
            <tr>
              <th className="px-3 py-2.5 font-medium">Model</th>
              <th className="px-3 py-2.5 font-medium">Tokens</th>
              <th className="px-3 py-2.5 font-medium">Latency</th>
              <th className="px-3 py-2.5 font-medium">TTFT</th>
              <th className="px-3 py-2.5 font-medium">Cost</th>
            </tr>
          </thead>
          <tbody>
            {history.length === 0 && (
              <tr>
                <td colSpan={5} className="px-3 py-8 text-center text-ink-500">
                  No requests yet for this session.
                </td>
              </tr>
            )}
            {history.map((r) => (
              <tr key={r.id} className="border-t border-ink-800/80 odd:bg-ink-900/30">
                <td className="px-3 py-2 font-mono text-xs text-foam">{r.model}</td>
                <td className="px-3 py-2 font-mono text-xs text-ink-300">
                  {formatTokens(r.prompt_tokens)}/{formatTokens(r.completion_tokens)}
                </td>
                <td className="px-3 py-2 font-mono text-xs text-ink-300">{r.duration_ms} ms</td>
                <td className="px-3 py-2 font-mono text-xs text-ink-300">{r.ttft_ms} ms</td>
                <td className="px-3 py-2 font-mono text-xs text-ember">{formatUsd(r.cost_usd)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function Metric({
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
