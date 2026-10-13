#[derive(Debug, Clone)]
pub struct TokenCounter {
    model: String,
    chars_per_token: f64,
}

impl TokenCounter {
    pub fn for_model(model: &str) -> Self {
        let normalized = model.trim().to_ascii_lowercase();
        // cxt intentionally keeps token accounting offline and dependency-light in 0.3.
        // These are conservative model-family profiles, not billable-token guarantees.
        let chars_per_token = match normalized.as_str() {
            "gpt-5" | "gpt5" | "gpt-5.1" | "gpt-5.2" | "gpt-5.6" => 3.55,
            "o200k" | "o200k_base" => 3.65,
            "cl100k" | "cl100k_base" => 3.75,
            "code" | "coding" => 3.45,
            _ => 4.0,
        };
        Self {
            model: normalized,
            chars_per_token,
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn count(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        ((text.chars().count() as f64) / self.chars_per_token).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_profiles_are_distinct() {
        let text = "fn main() { println!(\"hello world\"); }";
        let h = TokenCounter::for_model("heuristic").count(text);
        let g = TokenCounter::for_model("gpt-5").count(text);
        assert!(g >= h);
    }
}
