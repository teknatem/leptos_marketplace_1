use crate::domain::a017_llm_agent;
use crate::shared::llm::openai_provider::OpenAiProvider;
use crate::shared::llm::types::{ChatMessage, LlmProvider};
use contracts::domain::a024_bi_indicator::aggregate::{GenerateViewRequest, GenerateViewResponse};

const BI_VIEW_SYSTEM_PROMPT: &str = r#"You are a BI indicator visual designer. Your task is to generate HTML and CSS for a dashboard indicator card.

CONSTRAINTS:
- Output ONLY valid HTML and CSS. No JavaScript.
- The HTML will be placed inside a container: <div class="indicator-cell">...</div>
- Available placeholders (will be replaced with actual data at runtime):
  {{title}} — indicator title (e.g. "Выручка", "Маржа")
  {{value}} — main numeric value (e.g. "1 234 567 ₽", "42%")
  {{delta}} — change from previous period (e.g. "+5.2%", "−12 300")
- Allowed HTML tags: div, span, p, h1-h6, ul, ol, li, strong, em, b, i, small, sup, sub, table, thead, tbody, tr, td, th, br, hr, section, article, aside, header, footer
- Allowed attributes: class, style, id, title
- No <script>, no on* event attributes, no javascript: URIs
- CSS must be self-contained; use class names prefixed with `.bi-` to avoid conflicts
- The indicator should look professional and modern
- Use CSS variables for colors when possible: var(--bi-primary), var(--bi-success), var(--bi-danger), var(--bi-text), var(--bi-text-secondary), var(--bi-bg)

RESPONSE FORMAT (strictly follow):
```html
<your HTML here>
```

```css
<your CSS here>
```

EXPLANATION: <1-2 sentences about the design>

DESIGN GUIDELINES:
- Large prominent number for the main value
- Smaller secondary text for title and delta
- Use color to indicate positive/negative delta (green/red)
- Keep it clean and minimal — dashboard cards are typically 200-400px wide
- Support both light and dark backgrounds via CSS variables
"#;

/// Generate or refine indicator HTML/CSS using LLM
pub async fn generate_view(request: GenerateViewRequest) -> anyhow::Result<GenerateViewResponse> {
    let agent = if let Some(agent_id) = &request.agent_id {
        a017_llm_agent::service::get_by_id(agent_id).await?
    } else {
        a017_llm_agent::service::get_primary().await?
    };

    let agent = agent.ok_or_else(|| {
        anyhow::anyhow!("No LLM agent found. Configure an agent in a017_llm_agent first.")
    })?;

    let provider = if agent.api_endpoint.is_empty() {
        OpenAiProvider::new(
            agent.api_key.clone(),
            agent.model_name.clone(),
            agent.temperature,
            agent.max_tokens,
        )
    } else {
        OpenAiProvider::new_with_endpoint(
            agent.api_endpoint.clone(),
            agent.api_key.clone(),
            agent.model_name.clone(),
            agent.temperature,
            agent.max_tokens,
        )
    };

    let mut user_prompt = String::new();

    if let Some(html) = &request.current_html {
        if !html.trim().is_empty() {
            user_prompt.push_str(&format!("Current HTML:\n```html\n{}\n```\n\n", html));
        }
    }
    if let Some(css) = &request.current_css {
        if !css.trim().is_empty() {
            user_prompt.push_str(&format!("Current CSS:\n```css\n{}\n```\n\n", css));
        }
    }

    if !request.indicator_description.is_empty() {
        user_prompt.push_str(&format!("Indicator: {}\n\n", request.indicator_description));
    }

    user_prompt.push_str(&format!("Task: {}", request.prompt));

    let messages = vec![
        ChatMessage::system(BI_VIEW_SYSTEM_PROMPT),
        ChatMessage::user(user_prompt),
    ];

    let response = provider
        .chat_completion(messages)
        .await
        .map_err(|e| anyhow::anyhow!("LLM request failed: {}", e))?;

    parse_llm_response(&response.content)
}

fn parse_llm_response(content: &str) -> anyhow::Result<GenerateViewResponse> {
    let html = extract_code_block(content, "html").unwrap_or_default();
    let css = extract_code_block(content, "css").unwrap_or_default();

    let explanation = content
        .lines()
        .find(|line| line.starts_with("EXPLANATION:"))
        .map(|line| line.trim_start_matches("EXPLANATION:").trim().to_string())
        .unwrap_or_else(|| {
            content
                .lines()
                .rev()
                .find(|line| !line.trim().is_empty() && !line.starts_with("```"))
                .unwrap_or("Generated indicator view")
                .to_string()
        });

    if html.is_empty() && css.is_empty() {
        return Err(anyhow::anyhow!(
            "LLM did not return HTML/CSS in expected format. Raw response: {}",
            &content[..content.len().min(500)]
        ));
    }

    Ok(GenerateViewResponse {
        custom_html: html,
        custom_css: css,
        explanation,
    })
}

fn extract_code_block(content: &str, lang: &str) -> Option<String> {
    let marker_start = format!("```{}", lang);
    let start = content.find(&marker_start)?;
    let after_marker = start + marker_start.len();
    let rest = &content[after_marker..];
    let end = rest.find("```")?;
    let code = rest[..end].trim();
    if code.is_empty() {
        None
    } else {
        Some(code.to_string())
    }
}
