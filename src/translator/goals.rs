//! Translate ConcIR `BusinessGoal`s into [`crate::goals::GoalSpec`]s.
//!
//! Each [`concir::ast::BusinessGoal`] carries two dictionaries:
//!
//! * `marking`    — user-level place or resource name → expected token count
//! * `variables`  — CVN variable name → expected concrete value
//!
//! Keys are resolved against the **built net** by place name:
//!
//! * Keys of the form `"{fn}.{sid}"` map to the control place named `{fn}.{sid}`.
//! * Keys matching a resource name map to the resource place of that name.
//! * Keys starting with `cp_`, `rp_`, `wp_`, or `ra_` are treated as raw place
//!   references: the prefix is stripped and matched against place names; an
//!   unresolvable raw id is passed through as an always-unsatisfied predicate
//!   (the goal is reported unmet) rather than a warning.
//! * For resources whose *initial* token count is 0 (Channel and Condvar signal
//!   places), a requested count of 0 is interpreted as [`GoalPredicate::Empty`].
//!
//! Unrecognised keys produce a warning-level diagnostic (non-fatal:
//! [`translate_goals`] still returns the successfully-translated subset).

#![allow(clippy::collapsible_if)]

use std::collections::{HashMap, HashSet};

use concir::ast::{BaseType, ComplexBaseType, Program, Resource};
use serde_json::Value;
use unipn::{ConcreteVal, PlaceId};

use crate::goals::{GoalPredicate, GoalSpec};

/// Translate all business goals declared in `program` into goal specs over the
/// built `net`.
///
/// Returns `(specs, warnings)` where `warnings` lists goals that could not be
/// fully translated (e.g., unknown resource names).
pub fn translate_goals(program: &Program, net: &unipn::CvnNet) -> (Vec<GoalSpec>, Vec<String>) {
    let mut specs = Vec::new();
    let mut warnings = Vec::new();

    let resource_by_name: HashMap<&str, &Resource> =
        program.resources.iter().map(|r| (r.name.as_str(), r)).collect();
    let enum_variants: HashSet<String> = collect_enum_variants(program);
    let var_names: HashSet<&str> = program
        .resources
        .iter()
        .filter(|r| r.kind == "var")
        .map(|r| r.name.as_str())
        .collect();

    let place_by_name: HashMap<&str, PlaceId> = net
        .places
        .iter()
        .map(|p| (p.name.as_str(), p.id))
        .collect();

    for goal in &program.goals {
        let mut predicates = Vec::new();

        for (key, count) in &goal.marking {
            match marking_predicate(key, *count as usize, &resource_by_name, &place_by_name) {
                Ok(pred) => predicates.push(pred),
                Err(msg) => warnings.push(format!(
                    "goal '{}': marking key '{}' — {msg}",
                    goal.id, key
                )),
            }
        }

        for (var, value) in &goal.variables {
            // A misspelled variable would silently translate into a predicate
            // that no state can ever satisfy; reject it up front instead.
            if !var_names.contains(var.as_str()) {
                warnings.push(format!(
                    "goal '{}': variable '{}' — not a declared 'var' resource",
                    goal.id, var
                ));
                continue;
            }
            match variable_predicate(var, value, &enum_variants) {
                Ok(pred) => predicates.push(pred),
                Err(msg) => warnings.push(format!(
                    "goal '{}': variable '{}' — {msg}",
                    goal.id, var
                )),
            }
        }

        if predicates.is_empty() {
            warnings.push(format!(
                "goal '{}': no usable predicates produced (marking + variables)",
                goal.id
            ));
            continue;
        }

        specs.push(GoalSpec {
            id: goal.id.clone(),
            desc: goal.desc.clone(),
            predicates,
        });
    }

    (specs, warnings)
}

fn marking_predicate(
    key: &str,
    count: usize,
    resources: &HashMap<&str, &Resource>,
    place_by_name: &HashMap<&str, PlaceId>,
) -> Result<GoalPredicate, String> {
    // Raw place references (old-style prefixed ids). Prefix-strip and match
    // against place names; unresolvable ids are passed through as
    // always-unsatisfied predicates so the goal is reported unmet.
    if key.starts_with("cp_")
        || key.starts_with("rp_")
        || key.starts_with("wp_")
        || key.starts_with("ra_")
    {
        let stripped = strip_prefix(key);
        let place = place_by_name
            .get(stripped)
            .copied()
            .or_else(|| {
                // rp_<res> → resource named <res>.
                resources
                    .get(stripped)
                    .and_then(|_| place_by_name.get(stripped).copied())
            })
            .unwrap_or(PlaceId(place_by_name.len()));

        return Ok(reachability_predicate(place, count, false));
    }

    // "fn.sid" → control place named "{fn}.{sid}".
    if let Some(place) = place_by_name.get(key) {
        let starts_empty = resources
            .get(key)
            .map(|r| resource_starts_empty(r))
            .unwrap_or(false);
        return Ok(reachability_predicate(*place, count, starts_empty));
    }

    // Resource name → resource place.
    if let Some(res) = resources.get(key) {
        if let Some(place) = place_by_name.get(key) {
            return Ok(reachability_predicate(*place, count, resource_starts_empty(res)));
        }
    }

    Err("not a known resource, not a control-place reference, and not a raw place id".to_string())
}

/// Strip the `cp_`/`rp_`/`wp_`/`ra_` prefix (the remainder is matched against
/// place names).
fn strip_prefix(key: &str) -> &str {
    for p in ["cp_", "rp_", "wp_", "ra_"] {
        if let Some(rest) = key.strip_prefix(p) {
            return rest;
        }
    }
    key
}

/// Resources whose initial token count is 0.
///
/// For these places, a user goal of `count == 0` is meaningful as an
/// "empty / no residual" check. For all other resources (Mutex / RwLock /
/// Semaphore) the initial tokens are positive and `count == 0` just
/// degenerates to `tokens >= 0` which always holds, so we keep the standard
/// reachability semantics.
fn resource_starts_empty(res: &Resource) -> bool {
    matches!(
        (res.kind.as_str(), res.res_type.as_str()),
        ("sync", "Channel") | ("sync", "Condvar")
    )
}

fn reachability_predicate(place: PlaceId, count: usize, starts_empty: bool) -> GoalPredicate {
    if count == 0 && starts_empty {
        GoalPredicate::Empty { place }
    } else {
        GoalPredicate::Reachable {
            place,
            min_tokens: count,
        }
    }
}

fn variable_predicate(
    var: &str,
    value: &Value,
    enum_variants: &HashSet<String>,
) -> Result<GoalPredicate, String> {
    let concrete = json_to_concrete(value, enum_variants)?;
    Ok(GoalPredicate::GlobalEq {
        var: var.to_string(),
        value: concrete,
    })
}

fn json_to_concrete(v: &Value, enum_variants: &HashSet<String>) -> Result<ConcreteVal, String> {
    match v {
        Value::Bool(b) => Ok(ConcreteVal::Bool(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(ConcreteVal::Int(i))
            } else if let Some(f) = n.as_f64() {
                Ok(ConcreteVal::Float(f))
            } else {
                Err("number is neither i64 nor f64".to_string())
            }
        }
        Value::String(s) => {
            if enum_variants.contains(s.as_str()) {
                Ok(ConcreteVal::Enum(s.clone()))
            } else {
                Ok(ConcreteVal::Str(s.clone()))
            }
        }
        _ => Err("unsupported JSON value (expected bool / number / string)".to_string()),
    }
}

fn collect_enum_variants(program: &Program) -> HashSet<String> {
    let mut out = HashSet::new();
    for res in &program.resources {
        if let Some(BaseType::Complex(ComplexBaseType::Enum(variants))) = &res.base {
            for v in variants {
                out.insert(v.clone());
            }
        }
    }
    out
}
