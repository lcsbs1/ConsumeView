import { useCallback, useEffect, useState } from "react";
import { OverviewPanel } from "./components/OverviewPanel";
import { SessionPanel } from "./components/SessionPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { api, KeyStatus, onTokenLogged, ProxyStatus } from "./lib/api";

type Tab = "session" | "overview" | "settings";

export default function App() {
  const [tab, setTab] = useState<Tab>("session");
  const [proxy, setProxy] = useState<ProxyStatus | null>(null);
  const [keys, setKeys] = useState<KeyStatus | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  const refreshMeta = useCallback(async () => {
    const [p, k] = await Promise.all([api.getProxyStatus(), api.getKeyStatus()]);
    setProxy(p);
    setKeys(k);
  }, []);

  useEffect(() => {
    void refreshMeta().catch(console.error);
    const id = window.setInterval(() => {
      void api.getProxyStatus().then(setProxy).catch(() => undefined);
    }, 4000);
    return () => window.clearInterval(id);
  }, [refreshMeta]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void onTokenLogged(() => {
      setRefreshKey((n) => n + 1);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  return (
    <div className="mx-auto flex h-full max-w-6xl flex-col px-6 py-5">
      <header className="mb-6 flex flex-wrap items-center justify-between gap-4 animate-rise">
        <div>
          <p className="font-display text-3xl tracking-tight text-foam md:text-4xl">
            ConsumeView
          </p>
          <p className="mt-1 text-sm text-ink-300">
            Local token monitor for Gemini &amp; Grok through Cursor.
          </p>
        </div>
        <div className="flex items-center gap-3">
          <span className="inline-flex items-center gap-2 rounded-full border border-ink-700/80 bg-ink-900/50 px-3 py-1.5 text-xs text-ink-300">
            <span
              className={`h-2 w-2 rounded-full ${
                proxy?.listening ? "animate-pulse-dot bg-tide" : "bg-ink-500"
              }`}
            />
            {proxy?.listening ? "Listening" : "…"} · {proxy?.bind ?? "127.0.0.1:4000"}
          </span>
          <nav className="flex gap-1 rounded-lg bg-ink-900/70 p-1">
            {(
              [
                ["session", "Session"],
                ["overview", "Overview"],
                ["settings", "Settings"],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                onClick={() => setTab(id)}
                className={`rounded-md px-3 py-1.5 text-sm ${
                  tab === id ? "bg-ink-700 text-foam" : "text-ink-500 hover:text-ink-300"
                }`}
              >
                {label}
              </button>
            ))}
          </nav>
        </div>
      </header>

      <main className="min-h-0 flex-1 overflow-auto pb-4">
        {tab === "session" && <SessionPanel refreshKey={refreshKey} />}
        {tab === "overview" && <OverviewPanel refreshKey={refreshKey} />}
        {tab === "settings" && (
          <SettingsPanel
            proxy={proxy}
            keys={keys}
            onSaved={() => {
              void refreshMeta();
            }}
          />
        )}
      </main>
    </div>
  );
}
