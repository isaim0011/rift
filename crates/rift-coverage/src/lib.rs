//! Compares the operations in Discord's OpenAPI spec with the routes rift
//! implements (`coverage/implemented.txt`), so "how much of the API do we
//! cover?" is a number, not a feeling.
//!
//! Discord's spec is a **public preview** that "may not be correct"; its own
//! README says to follow the docs where they disagree. So this tool measures
//! coverage and catches drift (a route we call that the spec no longer lists);
//! it does not define what's right. Limits, flags and validation rules still
//! come from the docs.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

const METHODS: [&str; 5] = ["get", "put", "post", "delete", "patch"];

/// One operation in the spec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Op {
    /// Upper-case HTTP method.
    pub method: String,
    /// Path with every `{param}` normalized to `{}`.
    pub path: String,
    /// The spec's `operationId`, if any.
    pub id: String,
    /// Marked deprecated in the spec.
    pub deprecated: bool,
}

/// Replace every `{name}` with `{}` so differently named params still match.
pub fn normalize_path(p: &str) -> String {
    let mut out = String::with_capacity(p.len());
    let mut in_param = false;
    for c in p.chars() {
        match c {
            '{' => {
                in_param = true;
                out.push_str("{}");
            }
            '}' => in_param = false,
            _ if in_param => {}
            _ => out.push(c),
        }
    }
    out
}

/// Every `(method, path)` operation in the spec, sorted by path then method.
pub fn spec_operations(spec: &Value) -> Result<Vec<Op>, String> {
    let paths = spec
        .get("paths")
        .and_then(Value::as_object)
        .ok_or("the spec has no `paths` object")?;
    let mut ops = Vec::new();
    for (path, item) in paths {
        for m in METHODS {
            if let Some(op) = item.get(m) {
                ops.push(Op {
                    method: m.to_uppercase(),
                    path: normalize_path(path),
                    id: op.get("operationId").and_then(Value::as_str).unwrap_or("").to_owned(),
                    deprecated: op.get("deprecated").and_then(Value::as_bool).unwrap_or(false),
                });
            }
        }
    }
    ops.sort_by(|a, b| (&a.path, &a.method).cmp(&(&b.path, &b.method)));
    Ok(ops)
}

/// Parse `coverage/implemented.txt`: one `METHOD /path` per line, `#` comments.
pub fn parse_implemented(text: &str) -> Result<BTreeSet<(String, String)>, String> {
    let mut set = BTreeSet::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (Some(method), Some(path), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err(format!("line {}: expected `METHOD /path`, got {line:?}", n + 1));
        };
        let method = method.to_uppercase();
        if !METHODS.iter().any(|m| m.eq_ignore_ascii_case(&method)) {
            return Err(format!("line {}: unknown method {method:?}", n + 1));
        }
        if !path.starts_with('/') {
            return Err(format!("line {}: path must start with '/', got {path:?}", n + 1));
        }
        set.insert((method, normalize_path(path)));
    }
    Ok(set)
}

/// The result of comparing the spec with what we implement.
#[derive(Debug, Default)]
pub struct Report {
    /// Operations in the spec.
    pub total: usize,
    /// How many of them we implement.
    pub implemented: usize,
    /// resource -> (implemented, total), resource being the first path segment.
    pub by_resource: BTreeMap<String, (usize, usize)>,
    /// Spec operations we don't implement.
    pub missing: Vec<Op>,
    /// Routes we implement that the spec doesn't list (typo, removal, or an
    /// undocumented endpoint). Worth a look every time.
    pub unknown: Vec<(String, String)>,
}

/// The first path segment, e.g. `/channels/{}/messages` -> `channels`.
pub fn resource_of(path: &str) -> String {
    path.trim_start_matches('/').split('/').next().unwrap_or("").to_owned()
}

/// Compare.
pub fn analyze(ops: &[Op], implemented: &BTreeSet<(String, String)>) -> Report {
    let mut r = Report::default();
    let mut known: BTreeSet<(String, String)> = BTreeSet::new();
    for op in ops {
        let key = (op.method.clone(), op.path.clone());
        known.insert(key.clone());
        let entry = r.by_resource.entry(resource_of(&op.path)).or_insert((0, 0));
        entry.1 += 1;
        r.total += 1;
        if implemented.contains(&key) {
            entry.0 += 1;
            r.implemented += 1;
        } else {
            r.missing.push(op.clone());
        }
    }
    r.unknown = implemented.iter().filter(|k| !known.contains(*k)).cloned().collect();
    r
}

fn pct(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        100.0 * part as f64 / whole as f64
    }
}

/// A Markdown report.
pub fn render_markdown(r: &Report, spec_version: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# REST coverage vs Discord's OpenAPI spec\n");
    let _ = writeln!(
        s,
        "Spec version: `{spec_version}` (a public preview: it may be wrong and can change without notice; \
         where it disagrees with the docs, the docs win).\n"
    );
    let _ = writeln!(
        s,
        "**{} of {} operations implemented ({:.1}%).**\n",
        r.implemented,
        r.total,
        pct(r.implemented, r.total)
    );
    let _ = writeln!(s, "| Resource | Implemented | Total | % |\n|---|---:|---:|---:|");
    for (name, (done, total)) in &r.by_resource {
        let _ = writeln!(s, "| `{name}` | {done} | {total} | {:.0}% |", pct(*done, *total));
    }

    if !r.unknown.is_empty() {
        let _ = writeln!(s, "\n## Implemented but not in the spec\n");
        for (m, p) in &r.unknown {
            let _ = writeln!(s, "- `{m} {p}`");
        }
    }

    let _ = writeln!(s, "\n## Missing operations\n");
    let mut current = String::new();
    for op in &r.missing {
        let res = resource_of(&op.path);
        if res != current {
            let _ = writeln!(s, "\n### {res}\n");
            current = res;
        }
        let id = if op.id.is_empty() { String::new() } else { format!(" ({})", op.id) };
        let dep = if op.deprecated { " *deprecated*" } else { "" };
        let _ = writeln!(s, "- `{} {}`{id}{dep}", op.method, op.path);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec() -> Value {
        json!({
            "info": { "version": "10" },
            "paths": {
                "/channels/{channel_id}/messages": {
                    "parameters": [{ "name": "channel_id" }],
                    "get": { "operationId": "list_messages" },
                    "post": { "operationId": "create_message" }
                },
                "/channels/{channel_id}/messages/{message_id}": {
                    "delete": { "operationId": "delete_message", "deprecated": true }
                },
                "/gateway/bot": { "get": { "operationId": "get_bot_gateway" } },
                "/users/@me": { "head": {}, "get": {} }
            }
        })
    }

    #[test]
    fn param_names_are_normalized() {
        assert_eq!(
            normalize_path("/channels/{channel_id}/messages/{message_id}/reactions/{emoji_name}/@me"),
            "/channels/{}/messages/{}/reactions/{}/@me"
        );
        assert_eq!(normalize_path("/gateway/bot"), "/gateway/bot");
    }

    #[test]
    fn reads_operations_and_ignores_non_method_keys() {
        let ops = spec_operations(&spec()).unwrap();
        // `parameters` and `head` are not operations we track.
        assert_eq!(ops.len(), 5);
        let del = ops.iter().find(|o| o.method == "DELETE").unwrap();
        assert!(del.deprecated);
        assert_eq!(del.id, "delete_message");
        assert_eq!(del.path, "/channels/{}/messages/{}");
        assert!(spec_operations(&json!({})).is_err());
    }

    #[test]
    fn parses_the_implemented_list_with_comments_and_helpful_errors() {
        let ok = parse_implemented("# header\n\npost /channels/{id}/messages # create\nGET /gateway/bot\n").unwrap();
        assert!(ok.contains(&("POST".into(), "/channels/{}/messages".into())));
        assert_eq!(ok.len(), 2);

        assert!(parse_implemented("GET").unwrap_err().starts_with("line 1:"));
        assert!(parse_implemented("\nFETCH /x").unwrap_err().contains("line 2"));
        assert!(parse_implemented("GET x/y").unwrap_err().contains("must start with '/'"));
        assert!(parse_implemented("GET /a /b").is_err());
    }

    #[test]
    fn counts_coverage_missing_and_unknown_routes() {
        let ops = spec_operations(&spec()).unwrap();
        let implemented = parse_implemented(
            "POST /channels/{}/messages\nGET /gateway/bot\nGET /not/in/the/spec\n",
        )
        .unwrap();
        let r = analyze(&ops, &implemented);

        assert_eq!((r.implemented, r.total), (2, 5));
        assert_eq!(r.by_resource["channels"], (1, 3));
        assert_eq!(r.by_resource["gateway"], (1, 1));
        assert_eq!(r.missing.len(), 3);
        assert_eq!(r.unknown, vec![("GET".to_string(), "/not/in/the/spec".to_string())]);
    }

    #[test]
    fn the_report_says_the_numbers_and_lists_what_is_missing() {
        let ops = spec_operations(&spec()).unwrap();
        let r = analyze(&ops, &parse_implemented("GET /gateway/bot\n").unwrap());
        let md = render_markdown(&r, "10");
        assert!(md.contains("**1 of 5 operations implemented (20.0%).**"));
        assert!(md.contains("| `channels` | 0 | 3 | 0% |"));
        assert!(md.contains("- `DELETE /channels/{}/messages/{}` (delete_message) *deprecated*"));
        assert!(!md.contains("Implemented but not in the spec"));
    }

    #[test]
    fn our_real_implemented_list_is_well_formed() {
        // Guards the file the CLI reads by default.
        let set = parse_implemented(include_str!("../../../coverage/implemented.txt")).unwrap();
        assert!(set.len() >= 18, "only {} routes listed", set.len());
        assert!(set.contains(&("GET".into(), "/gateway/bot".into())));
        assert!(set.contains(&("POST".into(), "/interactions/{}/{}/callback".into())));
    }
}
