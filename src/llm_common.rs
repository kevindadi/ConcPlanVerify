//! Shared helpers for LLM integrations (`llm` feature).

/// Extract JSON content from an LLM response, handling markdown code fences.
pub fn extract_json_from_llm_response(response: &str) -> String {
    let trimmed = response.trim();
    if trimmed.starts_with("```") {
        let without_fence = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        let end = without_fence.rfind("```").unwrap_or(without_fence.len());
        without_fence[..end].trim().to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_json_fence() {
        let s = "```json\n{\"a\": 1}\n```";
        assert_eq!(extract_json_from_llm_response(s), "{\"a\": 1}");
    }

    #[test]
    fn plain_json_unchanged() {
        let s = "  {\"x\": true}  ";
        assert_eq!(extract_json_from_llm_response(s), "{\"x\": true}");
    }
}
