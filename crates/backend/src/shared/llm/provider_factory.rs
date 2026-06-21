use super::openai_provider::OpenAiProvider;
use super::openrouter_provider::{OpenRouterProvider, OPENROUTER_DEFAULT_ENDPOINT};
use super::types::{LlmError, LlmProvider};
use contracts::domain::a017_llm_agent::aggregate::{LlmAgent, LlmProviderType};

pub fn create_provider(
    agent: &LlmAgent,
    model_override: Option<&str>,
) -> Result<Box<dyn LlmProvider>, LlmError> {
    let model = model_override
        .filter(|m| !m.trim().is_empty())
        .unwrap_or(agent.model_name.as_str())
        .to_string();

    match agent.provider_type {
        LlmProviderType::OpenAI => Ok(Box::new(OpenAiProvider::new_with_endpoint(
            agent.api_endpoint.clone(),
            agent.api_key.clone(),
            model,
            agent.temperature,
            agent.max_tokens,
        ))),
        LlmProviderType::OpenRouter => Ok(Box::new(OpenRouterProvider::new(
            openrouter_endpoint(agent),
            agent.api_key.clone(),
            model,
            agent.temperature,
            agent.max_tokens,
        ))),
        _ => Err(LlmError::UnsupportedProvider(
            agent.provider_type.as_str().to_string(),
        )),
    }
}

pub async fn list_models(agent: &LlmAgent) -> Result<Vec<serde_json::Value>, LlmError> {
    match agent.provider_type {
        LlmProviderType::OpenAI => {
            let provider = OpenAiProvider::new_with_endpoint(
                agent.api_endpoint.clone(),
                agent.api_key.clone(),
                agent.model_name.clone(),
                agent.temperature,
                agent.max_tokens,
            );
            provider.list_models().await
        }
        LlmProviderType::OpenRouter => {
            let provider = OpenRouterProvider::new(
                openrouter_endpoint(agent),
                agent.api_key.clone(),
                agent.model_name.clone(),
                agent.temperature,
                agent.max_tokens,
            );
            provider.list_models().await
        }
        _ => Err(LlmError::UnsupportedProvider(
            agent.provider_type.as_str().to_string(),
        )),
    }
}

fn openrouter_endpoint(agent: &LlmAgent) -> String {
    if agent.api_endpoint.trim().is_empty() {
        OPENROUTER_DEFAULT_ENDPOINT.to_string()
    } else {
        agent.api_endpoint.clone()
    }
}
