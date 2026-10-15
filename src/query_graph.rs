//! Bounded evidence-guided reference traversal; never a compiler call graph.
use crate::bindings::{Binding, ScopeIndex};
use crate::dependency_index::ObservedPath;
use crate::{
    selection::Candidate,
    symbols::{ReferenceKind, SourceIndex, SourceSpan},
};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Evidence {
    ImportBound,
    LocalBound,
    UniqueName,
}

impl Evidence {
    fn label(self) -> &'static str {
        match self {
            Self::ImportBound => "import-bound",
            Self::LocalBound => "local-bound",
            Self::UniqueName => "unique-name-heuristic",
        }
    }
    fn cost(self) -> usize {
        match self {
            Self::ImportBound | Self::LocalBound => 1,
            Self::UniqueName => 4,
        }
    }
}

#[derive(Clone)]
struct Edge {
    target: usize,
    origin_file: usize,
    site: SourceSpan,
    kind: ReferenceKind,
    evidence: Evidence,
    reverse: bool,
}

impl Edge {
    fn cost(&self) -> usize {
        self.evidence.cost() + usize::from(self.kind == ReferenceKind::References)
    }
    fn description(&self, candidates: &[Candidate], root: &Path) -> String {
        let path = candidates[self.origin_file]
            .path
            .strip_prefix(root)
            .unwrap_or(&candidates[self.origin_file].path);
        let kind = match (self.kind, self.reverse) {
            (ReferenceKind::Calls, false) => "CALLS",
            (ReferenceKind::Calls, true) => "CALLED_BY",
            (ReferenceKind::References, false) => "REFERENCES",
            (ReferenceKind::References, true) => "REFERENCED_BY",
        };
        format!(
            "{}:{}:{kind}[{}]",
            path.to_string_lossy().replace('\\', "/"),
            self.site.start_line,
            self.evidence.label()
        )
    }
}

fn insert_edge(edges: &mut [BTreeMap<usize, Edge>], source: usize, edge: Edge) {
    let replace = edges[source]
        .get(&edge.target)
        .is_none_or(|old| (edge.cost(), edge.site.start_byte) < (old.cost(), old.site.start_byte));
    if replace {
        edges[source].insert(edge.target, edge);
    }
}

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
    root: &Path,
    scope: Option<&std::path::Path>,
    mut check: impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    let nodes: Vec<_> = indices
        .iter()
        .enumerate()
        .flat_map(|(file, index)| index.symbols.iter().map(move |symbol| (file, symbol)))
        .collect();
    let families: Vec<_> = candidates.iter().map(|c| family(&c.path)).collect();
    let mut definitions = BTreeMap::<_, Vec<usize>>::new();
    let mut file_definitions = BTreeMap::<_, Vec<usize>>::new();
    let scopes: Vec<_> = indices.iter().map(ScopeIndex::new).collect();
    let symbol_ids: BTreeMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(id, (_, s))| (s.id.as_str(), id))
        .collect();
    let candidate_paths: BTreeMap<_, _> = candidates
        .iter()
        .enumerate()
        .map(|(i, c)| (&c.path, i))
        .collect();
    for (id, (file, symbol)) in nodes.iter().enumerate() {
        if id % 256 == 0 {
            check()?;
        }
        if !matches!(families[*file], Some("py" | "ecmascript"))
            || !symbol.qualified_name.contains("::")
        {
            definitions
                .entry((families[*file], symbol.name.as_str()))
                .or_default()
                .push(id);
        }
        if !symbol.qualified_name.contains("::") {
            file_definitions
                .entry((*file, symbol.name.as_str()))
                .or_default()
                .push(id);
        }
    }
    let mut edges = vec![BTreeMap::new(); nodes.len()];
    let mut unresolved = BTreeMap::<usize, Vec<String>>::new();
    let mut imported_modules = BTreeMap::new();
    for (file, index) in indices.iter().enumerate() {
        for (binding_id, binding) in index.bindings.iter().enumerate() {
            check()?;
            let paths = crate::resolver::resolve_import_paths_scoped_checked(
                candidates[file]
                    .path
                    .parent()
                    .unwrap_or(std::path::Path::new("")),
                &binding.module,
                candidates[file]
                    .path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or(""),
                scope,
            )?;
            let paths: BTreeSet<_> = paths
                .into_iter()
                .filter(|p| crate::scope::check(scope, p).is_ok())
                .filter_map(|p| p.observed_canonicalize().ok())
                .collect();
            let mut targets = Vec::new();
            for path in paths {
                let Some(&target_file) = candidate_paths.get(&path) else {
                    continue;
                };
                if families[file] != families[target_file] {
                    continue;
                }
                targets.push(target_file);
            }
            imported_modules.insert((file, binding_id), targets);
        }
    }
    for (id, (file, symbol)) in nodes.iter().enumerate() {
        if id % 64 == 0 {
            check()?;
        }
        for site in &symbol.reference_sites {
            let name = &site.name;
            let (matches, evidence): (Vec<_>, _) = match scopes[*file].resolve(symbol, site) {
                Binding::Local(local) => (
                    symbol_ids
                        .get(local.id.as_str())
                        .copied()
                        .into_iter()
                        .collect(),
                    Evidence::LocalBound,
                ),
                Binding::Import { index, exported } => (
                    imported_modules
                        .get(&(*file, index))
                        .into_iter()
                        .flatten()
                        .flat_map(|target_file| {
                            let local = if families[*file] == Some("ecmascript") {
                                indices[*target_file]
                                    .exports
                                    .get(exported)
                                    .map(String::as_str)
                            } else {
                                Some(exported)
                            };
                            local.into_iter().flat_map(|local| {
                                file_definitions
                                    .get(&(*target_file, local))
                                    .into_iter()
                                    .flatten()
                                    .copied()
                            })
                        })
                        .collect(),
                    Evidence::ImportBound,
                ),
                Binding::Unbound => (
                    definitions
                        .get(&(families[*file], name.as_str()))
                        .cloned()
                        .unwrap_or_default(),
                    Evidence::UniqueName,
                ),
                Binding::Unresolved => continue,
            };
            // Ambiguous identities are not evidence. Do not fan out by common names.
            if matches.len() != 1 {
                if evidence == Evidence::ImportBound {
                    let messages = unresolved.entry(id).or_default();
                    if messages.len() < 4 {
                        let path = candidates[*file]
                            .path
                            .strip_prefix(root)
                            .unwrap_or(&candidates[*file].path);
                        messages.push(format!("query-unresolved:{name}@{}:{}: import has {} eligible definition targets; no global-name fallback", path.to_string_lossy().replace('\\', "/"), site.span.start_line, matches.len()));
                    }
                }
                continue;
            }
            let target = matches[0];
            if target != id && families[*file].is_some() {
                let edge = Edge {
                    target,
                    origin_file: *file,
                    site: site.span.clone(),
                    kind: site.kind,
                    evidence,
                    reverse: false,
                };
                insert_edge(&mut edges, id, edge.clone());
                insert_edge(
                    &mut edges,
                    target,
                    Edge {
                        target: id,
                        reverse: true,
                        ..edge
                    },
                );
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
        let mut pending: BinaryHeap<_> = seeds
            .iter()
            .map(|(id, _)| Reverse((0usize, 0usize, *id)))
            .collect();
        // Depth is part of the state: a cheaper two-hop path must not prevent
        // expanding a one-hop path to another useful implementation.
        let mut best = BTreeMap::<(usize, usize), (usize, Vec<String>)>::new();
        for (id, _) in &seeds {
            best.insert((*id, 0), (0, Vec::new()));
        }
        let mut visited = BTreeSet::new();
        while let Some(Reverse((cost, depth, id))) = pending.pop() {
            check()?;
            let Some((known_cost, path)) = best.get(&(id, depth)) else {
                continue;
            };
            if *known_cost != cost {
                continue;
            }
            let path = path.clone();
            if !visited.contains(&id) && visited.len() >= 64 {
                break;
            }
            visited.insert(id);
            let (target_file, symbol) = nodes[id];
            let candidate = &mut candidates[target_file];
            if let Some(messages) = unresolved.get(&id) {
                for message in messages {
                    if candidate
                        .reasons
                        .iter()
                        .filter(|r| r.starts_with("query-unresolved:"))
                        .count()
                        >= 8
                    {
                        break;
                    }
                    if !candidate.reasons.contains(message) {
                        candidate.reasons.push(message.clone());
                    }
                }
            }
            if candidate.query_symbols.len() < 24 {
                candidate
                    .query_symbols
                    .insert(symbol.qualified_name.clone());
                if depth > 0 {
                    candidate.score = candidate
                        .score
                        .max((score / (cost as i64 + 2)).clamp(1, 30));
                    let reason = format!(
                        "query-reference:depth={depth}:{}; cost={cost}; path={}",
                        symbol.qualified_name,
                        path.join(" -> ")
                    );
                    if !candidate.reasons.contains(&reason) {
                        candidate.reasons.push(reason);
                    }
                }
            }
            // Hubs (for example logging helpers) are not useful routing evidence.
            if edges[id].len() > 32 {
                let reason = format!(
                    "query-graph-limit:{}: {} neighbors; hub traversal suppressed",
                    symbol.qualified_name,
                    edges[id].len()
                );
                if !candidate.reasons.contains(&reason) {
                    candidate.reasons.push(reason);
                }
                continue;
            }
            if depth == 2 {
                continue;
            }
            for edge in edges[id].values() {
                if seeds.iter().any(|(seed, _)| *seed == edge.target) {
                    continue;
                }
                let next_cost = cost + edge.cost();
                let key = (edge.target, depth + 1);
                if best.get(&key).is_some_and(|(old, _)| *old <= next_cost) {
                    continue;
                }
                if !best.contains_key(&key) && best.len() >= 256 {
                    continue;
                }
                let mut next_path = path.clone();
                next_path.push(edge.description(candidates, root));
                best.insert(key, (next_cost, next_path));
                pending.push(Reverse((next_cost, depth + 1, edge.target)));
            }
        }
    }
    Ok(())
}
