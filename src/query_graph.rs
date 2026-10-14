//! Bounded, explainable identifier relationships; never a compiler call graph.
use crate::{selection::Candidate, symbols::SourceIndex};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

fn family(path: &std::path::Path) -> Option<&str> {
    let ext = path.extension()?.to_str()?;
    crate::tracer::get_language(ext)?;
    Some(match ext {
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" => "ecmascript",
        "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" | "hxx" | "ino" => "c-family",
        _ => ext,
    })
}

pub(crate) fn expand(
    candidates: &mut [Candidate],
    indices: &[SourceIndex],
    terms: &[String],
    mut check: impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    let nodes: Vec<_> = indices
        .iter()
        .enumerate()
        .flat_map(|(file, index)| index.symbols.iter().map(move |symbol| (file, symbol)))
        .collect();
    let families: Vec<_> = candidates.iter().map(|c| family(&c.path)).collect();
    let mut definitions = BTreeMap::<_, Vec<usize>>::new();
    for (id, (file, symbol)) in nodes.iter().enumerate() {
        if id % 256 == 0 {
            check()?;
        }
        definitions
            .entry((families[*file], symbol.name.as_str()))
            .or_default()
            .push(id);
    }
    let mut edges = vec![BTreeSet::new(); nodes.len()];
    for (id, (file, symbol)) in nodes.iter().enumerate() {
        if id % 64 == 0 {
            check()?;
        }
        for name in &symbol.references {
            let Some(matches) = definitions.get(&(families[*file], name.as_str())) else {
                continue;
            };
            // Ambiguous identities are not evidence. Do not fan out by common names.
            if matches.len() != 1 {
                continue;
            }
            let target = matches[0];
            if target != id && families[*file].is_some() {
                edges[id].insert(target);
                edges[target].insert(id);
            }
        }
    }
    let mut files: Vec<_> = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| c.score > 0)
        .map(|(id, c)| (id, c.score))
        .collect();
    files.sort_by(|(a, sa), (b, sb)| {
        sb.cmp(sa)
            .then_with(|| candidates[*a].path.cmp(&candidates[*b].path))
    });
    files.truncate(8);
    for (file, score) in files {
        check()?;
        let mut seeds: Vec<_> = nodes
            .iter()
            .enumerate()
            .filter(|(_, (f, _))| *f == file)
            .filter_map(|(id, (_, symbol))| {
                let names = crate::symbols::words(&symbol.qualified_name);
                let refs: BTreeSet<_> = symbol
                    .references
                    .iter()
                    .flat_map(|r| crate::symbols::words(r))
                    .collect();
                let weight = 10 * terms.iter().filter(|t| names.contains(t)).count()
                    + terms.iter().filter(|t| refs.contains(*t)).count();
                (weight > 0).then_some((id, weight))
            })
            .collect();
        seeds.sort_by(|(a, sa), (b, sb)| sb.cmp(sa).then_with(|| a.cmp(b)));
        seeds.truncate(3);
        let mut pending: VecDeque<_> = seeds.iter().map(|(id, _)| (*id, 0)).collect();
        let mut seen: BTreeSet<_> = seeds.iter().map(|(id, _)| *id).collect();
        while let Some((id, depth)) = pending.pop_front() {
            check()?;
            let (target_file, symbol) = nodes[id];
            let candidate = &mut candidates[target_file];
            if candidate.query_symbols.len() < 24 {
                candidate
                    .query_symbols
                    .insert(symbol.qualified_name.clone());
                if depth > 0 {
                    candidate.score = candidate
                        .score
                        .max((score / (depth as i64 + 2)).clamp(1, 30));
                    let reason = format!(
                        "query-reference:depth={depth}:{} (unique-name heuristic, not resolved)",
                        symbol.qualified_name
                    );
                    if !candidate.reasons.contains(&reason) {
                        candidate.reasons.push(reason);
                    }
                }
            }
            // Hubs (for example logging helpers) are not useful routing evidence.
            if depth == 2 || edges[id].len() > 32 {
                continue;
            }
            for next in &edges[id] {
                if seen.len() >= 64 {
                    break;
                }
                if seen.insert(*next) {
                    pending.push_back((*next, depth + 1));
                }
            }
        }
    }
    Ok(())
}
