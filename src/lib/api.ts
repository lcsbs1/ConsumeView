import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type ProxyStatus = {
  listening: boolean;
  bind: string;
};

export type KeyStatus = {
  gemini_set: boolean;
  xai_set: boolean;
};

export type SessionInfo = {
  id: string;
  name: string;
  created_at: string;
};

export type SessionTotals = {
  prompt_tokens: number;
  completion_tokens: number;
  cached_tokens: number;
  total_tokens: number;
  cost_usd: number;
  request_count: number;
  cache_reuse_rate: number;
  avg_ttft_ms: number;
  avg_tok_per_s: number;
};

export type StoredRequest = {
  id: string;
  session_id: string;
  provider: string;
  model: string;
  prompt_tokens: number;
  completion_tokens: number;
  cached_tokens: number;
  total_tokens: number;
  cost_usd: number;
  duration_ms: number;
  ttft_ms: number;
  created_at: string;
};

export type OverviewMetrics = {
  period: string;
  prompt_tokens: number;
  completion_tokens: number;
  cached_tokens: number;
  total_tokens: number;
  cost_usd: number;
  request_count: number;
  avg_ttft_ms: number;
  avg_tok_per_s: number;
  split: {
    gemini_cost_usd: number;
    grok_cost_usd: number;
    gemini_tokens: number;
    grok_tokens: number;
    gemini_requests: number;
    grok_requests: number;
  };
  projected_month_cost_usd: number;
};

export type TokenLoggedEvent = {
  id: string;
  session_id: string;
  provider: string;
  model: string;
  prompt_tokens: number;
  completion_tokens: number;
  cached_tokens: number;
  total_tokens: number;
  cost_usd: number;
  duration_ms: number;
  ttft_ms: number;
};

export const api = {
  getProxyStatus: () => invoke<ProxyStatus>("get_proxy_status"),
  getKeyStatus: () => invoke<KeyStatus>("get_key_status"),
  setApiKeys: (gemini?: string, xai?: string) =>
    invoke<void>("set_api_keys", { gemini: gemini ?? null, xai: xai ?? null }),
  listSessions: () => invoke<SessionInfo[]>("list_sessions"),
  createSession: (name: string) => invoke<SessionInfo>("create_session", { name }),
  getActiveSession: () => invoke<string>("get_active_session"),
  setActiveSession: (session_id: string) =>
    invoke<void>("set_active_session", { session_id }),
  getSessionTotals: (session_id: string) =>
    invoke<SessionTotals>("get_session_totals", { session_id }),
  getSessionHistory: (session_id: string, limit = 100) =>
    invoke<StoredRequest[]>("get_session_history", { session_id, limit }),
  getOverview: (period: "today" | "week" | "month") =>
    invoke<OverviewMetrics>("get_overview", { period }),
};

export function onTokenLogged(
  handler: (event: TokenLoggedEvent) => void,
): Promise<UnlistenFn> {
  return listen<TokenLoggedEvent>("token-logged", (e) => handler(e.payload));
}

export function formatUsd(n: number): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 4,
    maximumFractionDigits: 4,
  }).format(n || 0);
}

export function formatTokens(n: number): string {
  return new Intl.NumberFormat("en-US").format(n || 0);
}
