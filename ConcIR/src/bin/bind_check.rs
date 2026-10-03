//! Restricted binding-check CLI.
//!
//! Consumes the instrumenter's structural metadata (each resource's `display`
//! name, construction `site`, and for spawns the thread `entry` plus whether it
//! is unambiguous) and a CIR program, and emits per-resource verdicts:
//! `verified` / `unresolved` / `violated`. A versioned manifest may declare
//! bindings; each claim is checked, never trusted.
//!
//! Supported subset only: direct mutex/condvar/semaphore construction, channel
//! endpoints identified by an exact channel token, and spawns whose entry
//! function is unambiguous. Everything else is `unresolved`. This proves an
//! identity association only; it does NOT prove resource usage or whole-program
//! equivalence.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::process;

use serde_json::{json, Value};

fn binding_base(name: &str) -> String {
    for kind in ["mutex", "condvar", "semaphore", "channel", "atomic", "var"] {
        if let Some(idx) = name.rfind(&format!("_{kind}")) {
            let tail = &name[idx + kind.len() + 1..];
            if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
                return name[..idx].to_string();
            }
        }
    }
    name.to_string()
}

/// Channel token = the endpoint name with its endpoint suffix removed. Digits
/// are part of the identity (`ch1_tx` -> `ch1`, never `ch`). An endpoint with no
/// channel token (`tx`, `rx`) returns the empty string (unresolved).
fn channel_token(name: &str) -> String {
    for suf in ["_sender", "_receiver", "_tx", "_rx"] {
        if let Some(stripped) = name.strip_suffix(suf) {
            return stripped.to_string();
        }
    }
    String::new()
}

fn cir_resources(cir: &Value) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Some(mods) = cir.get("modules").and_then(Value::as_array) {
        for m in mods {
            let mname = m.get("name").and_then(Value::as_str).unwrap_or("");
            if let Some(rs) = m.get("resources").and_then(Value::as_array) {
                for r in rs {
                    let rname = r.get("name").and_then(Value::as_str).unwrap_or("");
                    let ty = r.get("type").and_then(Value::as_str).unwrap_or("");
                    out.insert(format!("{mname}::{rname}"), ty.to_string());
                }
            }
        }
    }
    out
}

fn cir_threads(cir: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(mods) = cir.get("modules").and_then(Value::as_array) {
        for m in mods {
            let mname = m.get("name").and_then(Value::as_str).unwrap_or("");
            if let Some(fns) = m.get("functions").and_then(Value::as_array) {
                for f in fns {
                    let fname = f.get("name").and_then(Value::as_str).unwrap_or("");
                    if fname != "main" {
                        out.push(format!("{mname}::{fname}"));
                    }
                }
            }
        }
    }
    out
}

fn resource_display(r: &Value) -> String {
    r.get("display")
        .and_then(Value::as_str)
        .unwrap_or_else(|| r.get("name").and_then(Value::as_str).unwrap_or(""))
        .to_string()
}

fn cir_capacity(cir: &Value) -> BTreeMap<String, (String, Option<i64>)> {
    let mut out = BTreeMap::new();
    let Some(mods) = cir.get("modules").and_then(Value::as_array) else { return out };
    for module in mods {
        let mname = module.get("name").and_then(Value::as_str).unwrap_or("");
        let Some(rs) = module.get("resources").and_then(Value::as_array) else { continue };
        for resource in rs {
            if resource.get("type").and_then(Value::as_str) != Some("Channel") {
                continue;
            }
            let rname = resource.get("name").and_then(Value::as_str).unwrap_or("");
            let fqn = format!("{mname}::{rname}");
            let class = match resource.get("capacity").and_then(Value::as_i64) {
                Some(0) => ("rendezvous".to_string(), Some(0)),
                Some(n) if n > 0 => ("bounded".to_string(), Some(n)),
                _ => ("unknown".to_string(), None),
            };
            out.insert(fqn, class);
        }
    }
    out
}

fn cir_semaphore_counts(cir: &Value) -> BTreeMap<String, Option<i64>> {
    let mut expected_count = BTreeMap::new();
    if let Some(mods) = cir.get("modules").and_then(Value::as_array) {
        for module in mods {
            let mname = module.get("name").and_then(Value::as_str).unwrap_or("");
            let Some(rs) = module.get("resources").and_then(Value::as_array) else { continue };
            for resource in rs {
                if resource.get("type").and_then(Value::as_str) != Some("Semaphore") {
                    continue;
                }
                let rname = resource.get("name").and_then(Value::as_str).unwrap_or("");
                expected_count.insert(
                    format!("{mname}::{rname}"),
                    resource.get("count").and_then(Value::as_i64),
                );
            }
        }
    }
    expected_count
}

fn init_at_site<'a>(inits: &'a [Value], site: &str) -> Option<&'a Value> {
    inits.iter().find(|item| item.get("site").and_then(Value::as_str) == Some(site))
}

/// Bind a CIR semaphore to the construction that is actually acquired or
/// released. An earlier unused `Semaphore::new` with the same name is not
/// evidence for that resource.
fn bind_used_semaphores(
    resources: &[Value],
    semantics: Option<&Value>,
    by_short: &BTreeMap<(String, String), Vec<String>>,
    verified: &mut BTreeMap<String, Value>,
    unresolved: &mut BTreeMap<String, Value>,
    used_cir: &mut BTreeSet<String>,
) {
    let inits = semantics
        .and_then(|s| s.get("semaphores"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut groups: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for resource in resources {
        if resource.get("kind").and_then(Value::as_str) != Some("Semaphore") {
            continue;
        }
        let short = binding_base(&resource_display(resource));
        groups.entry(short).or_default().push(resource);
    }
    for (short, group) in groups {
        let usable: Vec<&Value> = group
            .iter()
            .copied()
            .filter(|resource| {
                let site = resource.get("site").and_then(Value::as_str).unwrap_or("");
                init_at_site(&inits, site).is_some_and(|init| {
                    init.get("used").and_then(Value::as_bool) == Some(true)
                        && init.get("evidence").and_then(Value::as_str) == Some("literal_initial")
                })
            })
            .collect();
        if usable.len() != 1 {
            for resource in &group {
                let name = resource.get("name").and_then(Value::as_str).unwrap_or("");
                unresolved.insert(name.to_string(), json!({
                    "reason": "semaphore initial value must come from the single construction that is acquired or released",
                    "site": resource.get("site"),
                    "used_constructions": usable.len(),
                }));
            }
            continue;
        }
        let resource = usable[0];
        let name = resource.get("name").and_then(Value::as_str).unwrap_or("");
        let cands: Vec<String> = by_short
            .get(&(short.clone(), "Semaphore".to_string()))
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|c| !used_cir.contains(c))
            .collect();
        if cands.len() == 1 {
            used_cir.insert(cands[0].clone());
            verified.insert(name.to_string(), json!({"cir": cands[0], "rule": "used-construction"}));
        } else {
            unresolved.insert(name.to_string(), json!({
                "reason": "used semaphore construction does not match exactly one CIR semaphore",
                "candidates": cands,
                "site": resource.get("site"),
            }));
        }
        for resource in &group {
            let other = resource.get("name").and_then(Value::as_str).unwrap_or("");
            if other != name {
                unresolved.insert(other.to_string(), json!({
                    "reason": "unused semaphore construction is not the object acquired or released",
                    "site": resource.get("site"),
                }));
            }
        }
    }
}

fn semaphore_attributes(
    resources: &[Value],
    cir: &Value,
    verified: &BTreeMap<String, Value>,
    semantics: Option<&Value>,
) -> Vec<Value> {
    let inits = semantics
        .and_then(|s| s.get("semaphores"))
        .and_then(Value::as_array);
    let expected_count = cir_semaphore_counts(cir);
    let mut rows = Vec::new();
    for (cir_name, expected) in expected_count {
        let bound = verified.iter().find(|(_, info)| info.get("cir").and_then(Value::as_str) == Some(cir_name.as_str()));
        let Some((runtime, _)) = bound else {
            rows.push(json!({
                "resource_id": cir_name,
                "runtime": Value::Null,
                "expected": "semaphore_initial",
                "expected_capacity": expected,
                "observed": Value::Null,
                "observed_capacity": Value::Null,
                "construction_site": Value::Null,
                "evidence": "absent",
                "status": "unknown",
                "reason": "no used semaphore construction is bound to this CIR resource",
            }));
            continue;
        };
        let resource = resources.iter().find(|r| r.get("name").and_then(Value::as_str) == Some(runtime.as_str()));
        let site = resource.and_then(|r| r.get("site")).and_then(Value::as_str).unwrap_or("");
        let hit = inits.and_then(|items| init_at_site(items, site));
        let (status, observed, reason) = match hit {
            None => ("unknown", Value::Null, "bound semaphore has no construction fact at its site"),
            Some(item) if item.get("used").and_then(Value::as_bool) != Some(true) => {
                ("unknown", Value::Null, "bound semaphore is not the construction that is acquired or released")
            }
            Some(item) => match item.get("initial").and_then(Value::as_i64) {
                None => ("unknown", Value::Null, "semaphore initial count is not a literal"),
                Some(n) if expected == Some(n) => ("match", json!(n), "initial count of the used construction matches the CIR"),
                Some(n) => ("mismatch", json!(n), "initial count of the used construction differs from the CIR"),
            },
        };
        rows.push(json!({
            "resource_id": cir_name,
            "runtime": runtime,
            "expected": "semaphore_initial",
            "expected_capacity": expected,
            "observed": observed,
            "observed_capacity": observed,
            "construction_site": hit.and_then(|item| item.get("site")).cloned().unwrap_or(Value::Null),
            "evidence": hit.and_then(|item| item.get("evidence")).cloned().unwrap_or(json!("absent")),
            "status": status,
            "reason": reason,
        }));
    }
    rows
}

fn attribute_report(
    resources: &[Value],
    cir: &Value,
    verified: &BTreeMap<String, Value>,
    semantics: Option<&Value>,
) -> Vec<Value> {
    let kinds = cir_resources(cir);
    let caps = cir_capacity(cir);
    let endpoints = semantics
        .and_then(|s| s.get("endpoints"))
        .and_then(Value::as_array);
    let constructions = semantics
        .and_then(|s| s.get("constructions"))
        .and_then(Value::as_array);
    let mut rows = Vec::new();
    for (runtime, info) in verified {
        let cir_name = info.get("cir").and_then(Value::as_str).unwrap_or("");
        if kinds.get(cir_name).map(String::as_str) != Some("Channel") {
            continue;
        }
        let display = resources
            .iter()
            .find(|r| r.get("name").and_then(Value::as_str) == Some(runtime.as_str()))
            .map(resource_display)
            .unwrap_or_else(|| runtime.clone());
        let (expected, expected_capacity) = caps
            .get(cir_name)
            .cloned()
            .unwrap_or_else(|| ("unknown".to_string(), None));
        let base = json!({
            "resource_id": cir_name,
            "runtime": runtime,
            "expected": expected,
            "expected_capacity": expected_capacity,
        });
        let Some(endpoints) = endpoints else {
            let mut row = base;
            row["observed"] = Value::Null;
            row["observed_capacity"] = Value::Null;
            row["construction_site"] = Value::Null;
            row["evidence"] = json!("absent");
            row["status"] = json!("unknown");
            row["reason"] = json!("construction evidence absent; capacity is not assumed to match");
            rows.push(row);
            continue;
        };
        // Match the use-site name only. Do not look up another channel by the CIR name.
        let hits: Vec<&Value> = endpoints
            .iter()
            .filter(|e| e.get("name").and_then(Value::as_str) == Some(display.as_str()))
            .collect();
        if hits.is_empty() || hits.iter().any(|h| h.get("status").and_then(Value::as_str) != Some("traced")) {
            let mut row = base;
            row["observed"] = Value::Null;
            row["observed_capacity"] = Value::Null;
            row["construction_site"] = Value::Null;
            row["evidence"] = json!(hits.first().and_then(|h| h.get("evidence")).cloned().unwrap_or(json!("absent")));
            row["status"] = json!("unknown");
            row["reason"] = json!("no single traced construction for this endpoint");
            rows.push(row);
            continue;
        }
        let mut ids = BTreeSet::new();
        for hit in &hits {
            if let Some(id) = hit.get("construction_id").and_then(Value::as_str) {
                ids.insert(id.to_string());
            }
        }
        if ids.len() != 1 {
            let mut row = base;
            row["observed"] = Value::Null;
            row["observed_capacity"] = Value::Null;
            row["construction_site"] = Value::Null;
            row["evidence"] = json!("disagreeing_endpoints");
            row["status"] = json!("unknown");
            row["reason"] = json!("use sites of this name trace to different constructions");
            rows.push(row);
            continue;
        }
        let id = ids.iter().next().cloned().unwrap_or_default();
        let ctor = constructions.and_then(|list| {
            list.iter().find(|c| c.get("id").and_then(Value::as_str) == Some(id.as_str()))
        });
        let Some(ctor) = ctor else {
            let mut row = base;
            row["observed"] = Value::Null;
            row["observed_capacity"] = Value::Null;
            row["construction_site"] = Value::Null;
            row["evidence"] = json!("missing_construction");
            row["status"] = json!("unknown");
            row["reason"] = json!("traced construction id has no capacity record");
            rows.push(row);
            continue;
        };
        let observed = ctor.get("capacity_class").and_then(Value::as_str).unwrap_or("unknown").to_string();
        let observed_capacity = ctor.get("capacity").cloned().unwrap_or(Value::Null);
        let site = ctor.get("site").cloned().unwrap_or(Value::Null);
        let evidence = hits[0].get("evidence").cloned().unwrap_or(json!("let_tuple"));
        let caps_equal = match (expected_capacity, observed_capacity.as_i64()) {
            (Some(left), Some(right)) => left == right,
            (None, None) => true,
            _ => false,
        };
        let status = if expected == "unknown" || observed == "unknown" {
            "unknown"
        } else if expected == observed && caps_equal {
            "match"
        } else {
            "mismatch"
        };
        let reason = match status {
            "match" => format!("source {observed} matches CIR {expected}"),
            "mismatch" => format!(
                "source constructor is {observed} (capacity {observed_capacity}); CIR {cir_name} expects {expected} (capacity {expected_capacity:?})"
            ),
            _ => "capacity could not be determined; it is not treated as a match".to_string(),
        };
        let mut row = base;
        row["observed"] = json!(observed);
        row["observed_capacity"] = observed_capacity;
        row["construction_site"] = site;
        row["evidence"] = evidence;
        row["status"] = json!(status);
        row["reason"] = json!(reason);
        rows.push(row);
    }
    rows
}

fn check(resources: &[Value], cir: &Value, manifest: Option<&Value>, semantics: Option<&Value>) -> Value {
    let kinds = cir_resources(cir);
    let threads = cir_threads(cir);
    let mut by_short: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    let mut by_kind: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (fqn, kind) in &kinds {
        let short = fqn.rsplit("::").next().unwrap_or("").to_string();
        by_short.entry((short, kind.clone())).or_default().push(fqn.clone());
        by_kind.entry(kind.clone()).or_default().push(fqn.clone());
    }

    let mut display_counts: BTreeMap<String, usize> = BTreeMap::new();
    for r in resources {
        *display_counts.entry(resource_display(r)).or_insert(0) += 1;
    }

    // display -> runtime_id, for manifest alias resolution.
    let mut display_to_runtime: BTreeMap<String, String> = BTreeMap::new();
    for r in resources {
        let name = r.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        display_to_runtime.insert(resource_display(r), name);
    }

    let mut verified: BTreeMap<String, Value> = BTreeMap::new();
    let mut unresolved: BTreeMap<String, Value> = BTreeMap::new();
    let mut used: BTreeSet<String> = BTreeSet::new();

    for r in resources {
        let kind = r.get("kind").and_then(Value::as_str).unwrap_or("");
        let name = r.get("name").and_then(Value::as_str).unwrap_or("");
        if kind == "Spawn" || kind == "ChannelWrapper" || kind == "Semaphore" {
            continue;
        }
        let display = resource_display(r);
        if *display_counts.get(&display).unwrap_or(&0) > 1 {
            unresolved.insert(name.to_string(), json!({
                "reason": "duplicate runtime resource name", "site": r.get("site")}));
            continue;
        }
        let short = binding_base(&display);
        let cands: Vec<String> = by_short
            .get(&(short.clone(), kind.to_string()))
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|c| !used.contains(c))
            .collect();
        if cands.len() == 1 {
            used.insert(cands[0].clone());
            verified.insert(name.to_string(), json!({"cir": cands[0], "rule": "exact"}));
            continue;
        }
        if kind == "Channel" {
            let token = channel_token(&display);
            if !token.is_empty() {
                let hits: Vec<String> = by_kind
                    .get("Channel")
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|f| f.rsplit("::").next() == Some(token.as_str()))
                    .collect();
                if hits.len() == 1 {
                    verified.insert(name.to_string(),
                                    json!({"cir": hits[0], "rule": "channel-name"}));
                    continue;
                }
                unresolved.insert(name.to_string(), json!({
                    "reason": "channel token does not match exactly one CIR channel",
                    "token": token, "site": r.get("site")}));
                continue;
            }
            unresolved.insert(name.to_string(), json!({
                "reason": "channel endpoint has no channel token",
                "site": r.get("site")}));
            continue;
        }
        unresolved.insert(name.to_string(), json!({
            "reason": "no structural evidence", "candidates": cands,
            "site": r.get("site")}));
    }

    bind_used_semaphores(resources, semantics, &by_short, &mut verified, &mut unresolved, &mut used);

    for r in resources {
        if r.get("kind").and_then(Value::as_str) != Some("Spawn") {
            continue;
        }
        let name = r.get("name").and_then(Value::as_str).unwrap_or("");
        let entry = r.get("entry").and_then(Value::as_str);
        let unique = r.get("unique_entry").and_then(Value::as_bool).unwrap_or(false);
        if unique {
            if let Some(e) = entry {
                // A worker function may be activated at several creation sites;
                // each distinct runtime spawn instance binds to the same CIR
                // function. Do not reject the second legal instance.
                let hits: Vec<String> = threads
                    .iter()
                    .filter(|t| t.rsplit("::").next() == Some(e))
                    .cloned()
                    .collect();
                if hits.len() == 1 {
                    verified.insert(name.to_string(),
                                    json!({"cir": hits[0], "rule": "spawn-entry"}));
                    continue;
                }
            }
        }
        unresolved.insert(name.to_string(), json!({
            "reason": "thread entry is not unambiguous", "entry": entry,
            "site": r.get("site")}));
    }

    let mut violated: BTreeMap<String, Value> = BTreeMap::new();
    let mut claim_unresolved: BTreeMap<String, Value> = BTreeMap::new();
    if let Some(list) = manifest.and_then(Value::as_array) {
        for claim in list {
            let rust = claim.get("rust").and_then(Value::as_str).unwrap_or("");
            let cir = claim.get("cir").and_then(Value::as_str).unwrap_or("");
            // Normalise the claim key (runtime id or display) to one object.
            let runtime = if verified.contains_key(rust) || unresolved.contains_key(rust) {
                Some(rust.to_string())
            } else {
                display_to_runtime.get(rust).cloned()
            };
            let Some(k) = runtime else {
                // The object/target does not exist: an input error, not a
                // proven structural contradiction.
                violated.insert(rust.to_string(), json!({
                    "claim": cir, "reason": "no such runtime resource or display name"}));
                continue;
            };
            let actual: Option<String> = verified
                .get(&k)
                .and_then(|v| v.get("cir").and_then(Value::as_str))
                .map(|s| s.to_string());
            if actual.as_deref() == Some(cir) {
                continue; // established relation agrees with the claim
            }
            if verified.contains_key(&k) {
                // An established relation conflicts with the claim.
                verified.remove(&k);
                unresolved.remove(&k);
                violated.insert(rust.to_string(), json!({
                    "claim": cir, "actual": actual, "runtime": k,
                    "reason": "manifest conflicts with an established binding"}));
            } else {
                // A valid object whose relation is not established: unresolved,
                // not a contradiction. Not promoted by relaxing name matching.
                unresolved.remove(&k);
                claim_unresolved.insert(rust.to_string(), json!({
                    "claim": cir, "runtime": k,
                    "reason": "no structural evidence to establish the declared relation"}));
            }
        }
    }
    unresolved.extend(claim_unresolved);

    let mut attributes = attribute_report(resources, cir, &verified, semantics);
    attributes.extend(semaphore_attributes(resources, cir, &verified, semantics));
    let uncovered = semantics
        .and_then(|s| s.get("uncovered_sync"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    json!({
        "verified": verified,
        "unresolved": unresolved,
        "violated": violated,
        "attributes": attributes,
        "uncovered_sync": uncovered,
        "scope": "identity, channel-capacity attributes, and uncovered sync are separate; a conforming finite trace does not override a capacity conflict or an unknown capacity",
    })
}

fn die(msg: &str) -> ! {
    eprintln!("bind-check input error: {msg}");
    process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut resources_path = None;
    let mut cir_path = None;
    let mut manifest_path: Option<String> = None;
    let mut source_sha: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--resources" => { resources_path = args.get(i + 1).cloned(); i += 2; }
            "--cir" => { cir_path = args.get(i + 1).cloned(); i += 2; }
            "--manifest" => { manifest_path = args.get(i + 1).cloned(); i += 2; }
            "--source-sha256" => { source_sha = args.get(i + 1).cloned(); i += 2; }
            _ => { i += 1; }
        }
    }
    let (Some(rp), Some(cp)) = (resources_path, cir_path) else {
        die("usage: concir-bind-check --resources r.json --cir c.json [--manifest m.json]");
    };
    let resources_doc: Value = match fs::read_to_string(&rp) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| die(&format!("resources parse error: {e}"))),
        Err(e) => die(&format!("cannot read resources '{rp}': {e}")),
    };
    let resources = resources_doc.get("resources").and_then(Value::as_array).cloned()
        .unwrap_or_else(|| die("resources.json has no 'resources' array"));
    let cir: Value = match fs::read_to_string(&cp) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| die(&format!("cir parse error: {e}"))),
        Err(e) => die(&format!("cannot read cir '{cp}': {e}")),
    };

    // Optional source fingerprint: the instrumenter records the source path it
    // read; verify it matches the expected hash.
    if let Some(expected) = source_sha {
        let source_path = resources_doc.get("source").and_then(Value::as_str).unwrap_or("");
        let actual = fs::read(source_path).ok().map(|b| sha256_hex(&b));
        if actual.as_deref() != Some(expected.as_str()) {
            die("source fingerprint mismatch");
        }
    }

    // An explicit manifest must be well-formed; a broken input is an error, not
    // "no manifest".
    let manifest: Option<Value> = match manifest_path {
        None => None,
        Some(p) => {
            let text = fs::read_to_string(&p)
                .unwrap_or_else(|e| die(&format!("cannot read manifest '{p}': {e}")));
            let value: Value = serde_json::from_str(&text)
                .unwrap_or_else(|e| die(&format!("manifest parse error: {e}")));
            let arr = value.as_array().unwrap_or_else(|| die("manifest must be a JSON array"));
            for entry in arr {
                if entry.get("rust").and_then(Value::as_str).is_none()
                    || entry.get("cir").and_then(Value::as_str).is_none()
                {
                    die("manifest entry must have string 'rust' and 'cir'");
                }
            }
            Some(value)
        }
    };

    let semantics = resources_doc.get("channel_semantics");
    let result = check(&resources, &cir, manifest.as_ref(), semantics);
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}

fn sha256_hex(bytes: &[u8]) -> String {
    concir::hash::sha256_hex(bytes)
}
