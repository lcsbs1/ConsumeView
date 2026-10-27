//! Local OpenAI-compatible Axum proxy on 127.0.0.1:4000.

use crate::db::{MetricsDb, StoredRequest};
use crate::domain::{estimate_cost_usd, resolve_upstream, PricingTable, Provider};
use crate::secrets;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use bytes::Bytes;
use chrono::Utc;
use futures::StreamExt;
use serde_json::Value;
use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

pub const BIND_ADDR: &str = "127.0.0.1:4000";

#[derive(Clone)]
pub struct ProxyState {
    pub db: Arc<MetricsDb>,
    pub pricing: Arc<PricingTable>,
    pub app: AppHandle,
    pub listening: Arc<AtomicBool>,
    pub active_session: Arc<RwLock<String>>,
}

#[derive(Clone, serde::Serialize)]
pub struct TokenLoggedEvent {
    pub id: String,
    pub session_id: String,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cached_tokens: i64,
    pub total_tokens: i64,
    pub cost_usd: f64,
    pub duration_ms: i64,
    pub ttft_ms: i64,
}

pub fn build_router(state: ProxyState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health))
        .route("/v1/health", get(health))
        .route("/v1/chat/completions", post(chat_completions))
        .layer(cors)
        .with_state(state)
}

async fn health(State(state): State<ProxyState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "listening": state.listening.load(Ordering::Relaxed),
        "bind": BIND_ADDR,
    }))
}

async fn chat_completions(
    State(state): State<ProxyState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, (StatusCode, String)> {
    let started = Instant::now();
    let payload: Value = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")))?;

    let model = payload
        .get("model")
        .and_then(|m| m.as_str())
        .ok_or((StatusCode::BAD_REQUEST, "missing model".into()))?
        .to_string();

    let target = resolve_upstream(&model).ok_or((
        StatusCode::BAD_REQUEST,
        format!("unsupported model '{model}'; use gemini-* or grok-*"),
    ))?;

    let api_key = match target.provider {
        Provider::Gemini => secrets::get_gemini_key(),
        Provider::Grok => secrets::get_xai_key(),
    }
    .ok_or((
        StatusCode::UNAUTHORIZED,
        format!(
            "missing API key for provider {}",
            target.provider.as_str()
        ),
    ))?;

    let stream = payload
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let client = reqwest::Client::new();
    let mut req = client
        .post(target.url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .body(body.to_vec());

    if let Some(accept) = headers.get("accept") {
        req = req.header("Accept", accept.as_bytes());
    }

    let upstream = req
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("upstream error: {e}")))?;

    let status = StatusCode::from_u16(upstream.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);

    if !stream {
        let text = upstream
            .text()
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, format!("upstream read: {e}")))?;
        let usage = parse_usage_from_json(&text);
        let duration_ms = started.elapsed().as_millis() as i64;
        persist_and_emit(
            &state,
            &model,
            target.provider,
            usage,
            duration_ms,
            0,
        )
        .await;

        let mut resp = Response::new(Body::from(text));
        *resp.status_mut() = status;
        resp.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        return Ok(resp);
    }

    // Streaming SSE passthrough with usage tap
    let mut upstream_stream = upstream.bytes_stream();
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, Infallible>>(64);

    let state_bg = state.clone();
    let model_bg = model.clone();
    let provider = target.provider;

    tokio::spawn(async move {
        let mut buffer = String::new();
        let mut ttft_ms: i64 = 0;
        let mut saw_content = false;
        let mut usage = UsageCounts::default();

        while let Some(chunk) = upstream_stream.next().await {
            match chunk {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    if !saw_content {
                        if text.contains("\"content\"") || text.contains("delta") {
                            ttft_ms = started.elapsed().as_millis() as i64;
                            saw_content = true;
                        }
                    }
                    buffer.push_str(&text);
                    if let Some(u) = scrape_usage_from_sse(&buffer) {
                        usage = u;
                    }
                    let _ = tx.send(Ok(bytes)).await;
                }
                Err(_) => break,
            }
        }

        // Final usage may appear in last SSE data line
        if let Some(u) = scrape_usage_from_sse(&buffer) {
            usage = u;
        }

        let duration_ms = started.elapsed().as_millis() as i64;
        persist_and_emit(&state_bg, &model_bg, provider, usage, duration_ms, ttft_ms).await;
    });

    let body_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    let mut resp = Response::new(Body::from_stream(body_stream));
    *resp.status_mut() = status;
    resp.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    resp.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
    Ok(resp)
}

#[derive(Debug, Clone, Default)]
struct UsageCounts {
    prompt_tokens: i64,
    completion_tokens: i64,
    cached_tokens: i64,
    total_tokens: i64,
}

fn parse_usage_from_json(text: &str) -> UsageCounts {
    let Ok(v) = serde_json::from_str::<Value>(text) else {
        return UsageCounts::default();
    };
    usage_from_value(&v)
}

fn scrape_usage_from_sse(buffer: &str) -> Option<UsageCounts> {
    // Walk data lines; keep last usage found
    let mut last = None;
    for line in buffer.lines() {
        let line = line.trim();
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data == "[DONE]" || data.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(data) {
            if v.get("usage").is_some() {
                last = Some(usage_from_value(&v));
            }
        }
    }
    last
}

fn usage_from_value(v: &Value) -> UsageCounts {
    let usage = v.get("usage").cloned().unwrap_or(Value::Null);
    let prompt = usage
        .get("prompt_tokens")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    let completion = usage
        .get("completion_tokens")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    let total = usage
        .get("total_tokens")
        .and_then(|x| x.as_i64())
        .unwrap_or(prompt + completion);
    let cached = usage
        .pointer("/prompt_tokens_details/cached_tokens")
        .and_then(|x| x.as_i64())
        .or_else(|| usage.get("cached_tokens").and_then(|x| x.as_i64()))
        .unwrap_or(0);
    UsageCounts {
        prompt_tokens: prompt,
        completion_tokens: completion,
        cached_tokens: cached,
        total_tokens: total,
    }
}

async fn persist_and_emit(
    state: &ProxyState,
    model: &str,
    provider: Provider,
    usage: UsageCounts,
    duration_ms: i64,
    ttft_ms: i64,
) {
    let rates = state.pricing.rates_for(model);
    let cost = rates
        .map(|r| {
            estimate_cost_usd(
                r,
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.cached_tokens,
            )
        })
        .unwrap_or(0.0);

    let session_id = state.active_session.read().await.clone();
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let stored = StoredRequest {
        id: id.clone(),
        session_id: session_id.clone(),
        provider: provider.as_str().to_string(),
        model: model.to_string(),
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        cached_tokens: usage.cached_tokens,
        total_tokens: usage.total_tokens,
        cost_usd: cost,
        duration_ms,
        ttft_ms,
        created_at,
    };

    if let Err(e) = state.db.insert_request(&stored) {
        tracing::error!("failed to persist request: {e}");
    }

    let event = TokenLoggedEvent {
        id: stored.id,
        session_id: stored.session_id,
        provider: stored.provider,
        model: stored.model,
        prompt_tokens: stored.prompt_tokens,
        completion_tokens: stored.completion_tokens,
        cached_tokens: stored.cached_tokens,
        total_tokens: stored.total_tokens,
        cost_usd: stored.cost_usd,
        duration_ms: stored.duration_ms,
        ttft_ms: stored.ttft_ms,
    };
    let _ = state.app.emit("token-logged", event);
}

pub async fn serve(state: ProxyState) -> anyhow::Result<()> {
    let app = build_router(state.clone());
    let listener = tokio::net::TcpListener::bind(BIND_ADDR).await?;
    state.listening.store(true, Ordering::Relaxed);
    tracing::info!("ConsumeView proxy listening on http://{BIND_ADDR}");
    axum::serve(listener, app).await?;
    state.listening.store(false, Ordering::Relaxed);
    Ok(())
}
