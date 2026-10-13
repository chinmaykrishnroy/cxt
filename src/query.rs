use crate::selection::Candidate;
use std::fs;
use std::path::Path;

pub fn rank(candidates: Vec<Candidate>, query: &str, root: &Path) -> Vec<Candidate> {
    rank_checked(candidates, query, root, || Ok(())).unwrap_or_default()
}

pub(crate) fn rank_checked<F>(
    mut candidates: Vec<Candidate>,
    query: &str,
    root: &Path,
    mut check: F,
) -> Result<Vec<Candidate>, String>
where
    F: FnMut() -> Result<(), String>,
{
    check()?;
    let terms = query_terms(query);
    if terms.is_empty() {
        return Ok(candidates);
    }

    for candidate in &mut candidates {
        check()?;
        let path_text = candidate
            .path
            .strip_prefix(root)
            .unwrap_or(&candidate.path)
            .to_string_lossy()
            .to_ascii_lowercase();
        let file_name = candidate
            .path
            .file_name()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let mut score = 0i64;
        let mut matched_path = Vec::new();
        for term in &terms {
            if file_name.contains(term.as_str()) {
                score += 30;
                matched_path.push(term.clone());
            } else if path_text.contains(term.as_str()) {
                score += 15;
                matched_path.push(term.clone());
            }
        }
        if !matched_path.is_empty() {
            candidate
                .reasons
                .push(format!("query-path:{}", matched_path.join(",")));
        }

        if let Ok(content) = fs::read_to_string(&candidate.path) {
            let lower = content.to_ascii_lowercase();
            let mut content_matches = 0usize;
            for term in &terms {
                let count = lower.match_indices(term.as_str()).take(8).count();
                if count > 0 {
                    content_matches += 1;
                    score += 6 + (count.min(5) as i64 * 2);
                }
            }
            if content_matches > 0 {
                candidate
                    .reasons
                    .push(format!("query-content:{}-terms", content_matches));
            }
        }
        candidate.score += score;
    }

    candidates.retain(|c| c.score > 0);
    candidates.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    check()?;
    Ok(candidates)
}

fn query_terms(query: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the", "and", "for", "with", "from", "this", "that", "why", "does", "when", "what",
        "where", "into", "fail", "fails", "failing", "issue", "bug", "error", "code", "file",
    ];
    let mut terms: Vec<String> = Vec::new();
    for token in query
        .to_ascii_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
    {
        if token.len() >= 3 && !STOP.contains(&token) && !terms.iter().any(|t| t.as_str() == token)
        {
            terms.push(token.to_string());
        }
    }
    terms
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn filename_match_ranks() {
        let c = Candidate::new(PathBuf::from("src/volume_slider.rs"), "scan");
        let ranked = rank(vec![c], "volume slider sync", Path::new("."));
        assert_eq!(ranked.len(), 1);
        assert!(ranked[0].score > 0);
    }

    #[test]
    fn checkout_directory_does_not_affect_relevance() {
        let root = Path::new("context-budget-checkout");
        let candidate = Candidate::new(root.join("LICENSE"), "scan");
        assert!(rank(vec![candidate], "context budget", root).is_empty());
    }

    #[test]
    fn checked_ranking_propagates_scan_cancellation() {
        let candidate = Candidate::new(PathBuf::from("app.rs"), "scan");
        let error = rank_checked(vec![candidate], "app", Path::new("."), || {
            Err("Scan stopped: time limit reached".into())
        })
        .unwrap_err();
        assert!(error.contains("time limit"));
    }
}
