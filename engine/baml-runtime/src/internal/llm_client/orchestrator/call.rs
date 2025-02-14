use anyhow::Result;
use baml_types::BamlValue;
use internal_baml_core::ir::repr::IntermediateRepr;
use jsonish::BamlValueWithFlags;
use web_time::Duration;

use crate::{
    internal::{
        llm_client::{
            parsed_value_to_response,
            traits::{WithClientProperties, WithPrompt, WithSingleCallable},
            LLMResponse, ResponseBamlValue,
        },
        prompt_renderer::PromptRenderer,
    },
    RuntimeContext,
};

use super::{OrchestrationScope, OrchestratorNodeIterator};

pub async fn orchestrate(
    iter: OrchestratorNodeIterator,
    ir: &IntermediateRepr,
    ctx: &RuntimeContext,
    prompt: &PromptRenderer,
    params: &BamlValue,
    parse_fn: impl Fn(&str) -> Result<BamlValueWithFlags>,
) -> (
    Vec<(
        OrchestrationScope,
        LLMResponse,
        Option<Result<BamlValueWithFlags>>,
        Option<Result<ResponseBamlValue>>,
    )>,
    Duration,
) {
    let mut results = Vec::new();
    let mut total_sleep_duration = std::time::Duration::from_secs(0);

    for node in iter {
        let prompt = match node.render_prompt(ir, prompt, ctx, params).await {
            Ok(p) => p,
            Err(e) => {
                results.push((
                    node.scope,
                    LLMResponse::InternalFailure(e.to_string()),
                    None,
                    None,
                ));
                continue;
            }
        };
        let response = node.single_call(ctx, &prompt).await;
        let parsed_response = match &response {
            LLMResponse::Success(s) => {
                if !node
                    .finish_reason_filter()
                    .is_allowed(s.metadata.finish_reason.as_ref())
                {
                    Some(Err(anyhow::anyhow!(crate::errors::ExposedError::FinishReasonError {
                        prompt: prompt.to_string(),
                        raw_output: s.content.clone(),
                        message: "Finish reason not allowed".to_string(),
                        finish_reason: s.metadata.finish_reason.clone(),
                    })))
                } else {
                    Some(parse_fn(&s.content))
                }
            },
            _ => None,
        };

        let sleep_duration = node.error_sleep_duration().cloned();
        let (parsed_response, response_with_constraints) = match parsed_response {
            Some(Ok(v)) => (Some(Ok(v.clone())), Some(Ok(parsed_value_to_response(&v)))),
            Some(Err(e)) => (None, Some(Err(e))),
            None => (None, None),
        };
        results.push((
            node.scope,
            response,
            parsed_response,
            response_with_constraints,
        ));

        // Currently, we break out of the loop if an LLM responded, even if we couldn't parse the result.
        if results
            .last()
            .map_or(false, |(_, r, _, _)| matches!(r, LLMResponse::Success(_)))
        {
            break;
        } else if let Some(duration) = sleep_duration {
            total_sleep_duration += duration;
            async_std::task::sleep(duration).await;
        }
    }

    (results, total_sleep_duration)
}
