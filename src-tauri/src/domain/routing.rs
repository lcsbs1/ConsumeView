//! Model prefix routing for Gemini and Grok.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Gemini,
    Grok,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Gemini => "gemini",
            Provider::Grok => "grok",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamTarget {
    pub provider: Provider,
    pub url: &'static str,
}

const XAI_CHAT: &str = "https://api.x.ai/v1/chat/completions";
const GEMINI_OPENAI_CHAT: &str =
    "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions";

/// Route by model name prefix. Returns `None` for unsupported models.
pub fn resolve_upstream(model: &str) -> Option<UpstreamTarget> {
    let lower = model.to_ascii_lowercase();
    if lower.starts_with("grok-") {
        Some(UpstreamTarget {
            provider: Provider::Grok,
            url: XAI_CHAT,
        })
    } else if lower.starts_with("gemini-") {
        Some(UpstreamTarget {
            provider: Provider::Gemini,
            url: GEMINI_OPENAI_CHAT,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_grok_and_gemini() {
        let g = resolve_upstream("grok-2").unwrap();
        assert_eq!(g.provider, Provider::Grok);
        assert_eq!(g.url, XAI_CHAT);

        let m = resolve_upstream("gemini-2.0-flash").unwrap();
        assert_eq!(m.provider, Provider::Gemini);
        assert_eq!(m.url, GEMINI_OPENAI_CHAT);
    }

    #[test]
    fn rejects_unknown() {
        assert!(resolve_upstream("gpt-4o").is_none());
        assert!(resolve_upstream("claude-3").is_none());
    }
}
