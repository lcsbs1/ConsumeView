import { FormEvent, useState } from "react";
import { api, KeyStatus, ProxyStatus } from "../lib/api";

type Props = {
  proxy: ProxyStatus | null;
  keys: KeyStatus | null;
  onSaved: () => void;
};

export function SettingsPanel({ proxy, keys, onSaved }: Props) {
  const [gemini, setGemini] = useState("");
  const [xai, setXai] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setMessage(null);
    try {
      await api.setApiKeys(
        gemini.trim() ? gemini.trim() : undefined,
        xai.trim() ? xai.trim() : undefined,
      );
      setGemini("");
      setXai("");
      setMessage("Keys stored in the OS keyring (local fallback if unavailable).");
      onSaved();
    } catch (err) {
      setMessage(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="animate-rise space-y-6">
      <header>
        <h2 className="font-display text-2xl text-foam">Settings</h2>
        <p className="mt-1 max-w-xl text-sm text-ink-300">
          Paste provider keys here only. They stay on this machine — never sent to Cursor.
        </p>
      </header>

      <div className="flex flex-wrap items-center gap-4 text-sm">
        <span className="inline-flex items-center gap-2 rounded-full bg-tide-soft px-3 py-1.5 text-tide">
          <span
            className={`h-2 w-2 rounded-full bg-tide ${proxy?.listening ? "animate-pulse-dot" : "opacity-40"}`}
          />
          {proxy?.listening ? "Proxy listening" : "Proxy starting…"}
        </span>
        <code className="font-mono text-ink-300">{proxy?.bind ?? "127.0.0.1:4000"}</code>
        <span className="text-ink-500">
          Gemini {keys?.gemini_set ? "set" : "missing"} · xAI {keys?.xai_set ? "set" : "missing"}
        </span>
      </div>

      <form onSubmit={onSubmit} className="max-w-xl space-y-4">
        <label className="block space-y-1.5">
          <span className="text-xs uppercase tracking-[0.14em] text-ink-500">
            Gemini API key
          </span>
          <input
            type="password"
            autoComplete="off"
            value={gemini}
            onChange={(e) => setGemini(e.target.value)}
            placeholder={keys?.gemini_set ? "•••••••• (leave blank to keep)" : "AIza…"}
            className="w-full rounded-lg border border-ink-700 bg-ink-900/80 px-3 py-2.5 font-mono text-sm text-foam outline-none ring-tide/40 placeholder:text-ink-500 focus:ring-2"
          />
        </label>
        <label className="block space-y-1.5">
          <span className="text-xs uppercase tracking-[0.14em] text-ink-500">
            xAI API key
          </span>
          <input
            type="password"
            autoComplete="off"
            value={xai}
            onChange={(e) => setXai(e.target.value)}
            placeholder={keys?.xai_set ? "•••••••• (leave blank to keep)" : "xai-…"}
            className="w-full rounded-lg border border-ink-700 bg-ink-900/80 px-3 py-2.5 font-mono text-sm text-foam outline-none ring-tide/40 placeholder:text-ink-500 focus:ring-2"
          />
        </label>
        <button
          type="submit"
          disabled={busy || (!gemini.trim() && !xai.trim())}
          className="rounded-lg bg-ember px-4 py-2.5 text-sm font-medium text-ink-950 transition hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-40"
        >
          {busy ? "Saving…" : "Save keys"}
        </button>
        {message && <p className="text-sm text-ink-300">{message}</p>}
      </form>
    </section>
  );
}
