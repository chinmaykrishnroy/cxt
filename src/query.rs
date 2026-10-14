use crate::selection::Candidate;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    clap::ValueEnum,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Ranker {
    #[default]
    Hybrid,
    Lexical,
    Symbols,
    Legacy,
}

pub fn rank(candidates: Vec<Candidate>, query: &str, root: &Path) -> Vec<Candidate> {
    rank_checked(candidates, query, root, || Ok(())).unwrap_or_default()
}

pub(crate) fn rank_checked<F>(
    candidates: Vec<Candidate>,
    query: &str,
    root: &Path,
    check: F,
) -> Result<Vec<Candidate>, String>
where
    F: FnMut() -> Result<(), String>,
{
    rank_with_store_checked(
        candidates,
        query,
        root,
        &crate::index::IndexStore::new(None),
        Ranker::Hybrid,
        true,
        check,
    )
}

pub(crate) fn rank_with_store_checked<F>(
    mut candidates: Vec<Candidate>,
    query: &str,
    root: &Path,
    store: &crate::index::IndexStore,
    ranker: Ranker,
    expand: bool,
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

    let mut documents = Vec::new();
    let mut legacy_scores = Vec::new();
    let mut indices = Vec::new();
    let mut frequencies = BTreeMap::<String, usize>::new();
    for candidate in &candidates {
        check()?;
        let content = fs::read_to_string(&candidate.path).unwrap_or_default();
        let words = crate::symbols::words(&content);
        let term_counts: BTreeMap<_, _> = terms
            .iter()
            .map(|term| {
                (
                    term.clone(),
                    words.iter().filter(|word| *word == term).count(),
                )
            })
            .collect();
        for (term, count) in &term_counts {
            if *count > 0 {
                *frequencies.entry(term.clone()).or_default() += 1;
            }
        }
        documents.push((words.len(), term_counts));
        legacy_scores.push(if ranker == Ranker::Legacy {
            let lower = content.to_lowercase();
            terms
                .iter()
                .map(|term| {
                    let n = lower.match_indices(term).take(8).count();
                    if n > 0 {
                        6 + 2 * n.min(5) as i64
                    } else {
                        0
                    }
                })
                .sum::<i64>()
        } else {
            0
        });
        indices.push(store.load(&candidate.path, &content).index);
    }
    let average = (documents.iter().map(|(length, _)| *length).sum::<usize>() as f64
        / documents.len().max(1) as f64)
        .max(1.0);
    let count = candidates.len() as f64;
    for (i, candidate) in candidates.iter_mut().enumerate() {
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
        if !matched_path.is_empty() && ranker != Ranker::Symbols {
            candidate
                .reasons
                .push(format!("query-path:{}", matched_path.join(",")));
        }

        let mut bm25 = 0.0;
        for term in &terms {
            let frequency = *documents[i].1.get(term).unwrap_or(&0) as f64;
            let df = *frequencies.get(term).unwrap_or(&0) as f64;
            if frequency > 0.0 {
                let idf = (1.0 + (count - df + 0.5) / (df + 0.5)).ln();
                bm25 += idf * frequency * 2.2
                    / (frequency + 1.2 * (0.25 + 0.75 * documents[i].0 as f64 / average));
            }
        }
        let symbol_matches = indices[i]
            .symbols
            .iter()
            .map(|s| {
                let words = crate::symbols::words(&s.qualified_name);
                terms.iter().filter(|t| words.contains(t)).count()
            })
            .max()
            .unwrap_or(0);
        match ranker {
            Ranker::Hybrid => score += (bm25 * 12.0).round() as i64 + symbol_matches as i64 * 24,
            Ranker::Lexical => score += (bm25 * 12.0).round() as i64,
            Ranker::Symbols => score = symbol_matches as i64 * 24,
            Ranker::Legacy => score += legacy_scores[i],
        }
        candidate.reasons.push(format!("ranker:{ranker:?}"));
        if bm25 > 0.0 && matches!(ranker, Ranker::Hybrid | Ranker::Lexical) {
            candidate.reasons.push(format!("query-bm25:{:.3}", bm25));
        }
        if symbol_matches > 0 && matches!(ranker, Ranker::Hybrid | Ranker::Symbols) {
            candidate
                .reasons
                .push(format!("query-symbol:{symbol_matches}-terms"));
        }
        candidate.score += score;
    }

    let mut seeds: Vec<_> = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| c.score > 0)
        .map(|(i, c)| (i, c.score))
        .collect();
    seeds.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    seeds.truncate(8);
    let mut graph_boost = vec![0; candidates.len()];
    for (seed, value) in seeds
        .into_iter()
        .filter(|_| expand && ranker == Ranker::Hybrid)
    {
        check()?;
        for (i, index) in indices.iter().enumerate() {
            if i % 64 == 0 {
                check()?;
            }
            if i != seed
                && index
                    .symbols
                    .iter()
                    .any(|s| indices[seed].identifiers.contains(&s.name))
            {
                graph_boost[i] = graph_boost[i].max((value / 3).min(30));
            }
        }
    }
    for (c, boost) in candidates.iter_mut().zip(graph_boost) {
        if boost > 0 {
            c.score += boost;
            c.reasons
                .push("query-lexical-reference (not resolved call graph)".into());
        }
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
    for token in crate::symbols::words(query) {
        if token.len() >= 2 && !STOP.contains(&token.as_str()) && !terms.contains(&token) {
            terms.push(token);
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
