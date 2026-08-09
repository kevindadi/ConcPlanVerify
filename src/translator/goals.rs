//! Translate ConcIR `BusinessGoal`s into CVN [`GoalSpec`]s.
//!
//! Each [`concir::ast::BusinessGoal`] carries two dictionaries:
//!
//! * `marking`    — user-level place or resource name → expected token count
//! * `variables`  — CVN variable name → expected concrete value
//!
//! The translator resolves these into CVN predicates using the same naming
//! convention as [`super::context`]:
//!
//! * Keys matching a ConcIR resource name are mapped to the resource place
//!   `rp_{name}`. For resources whose *initial* token count is 0 (Channel
//!   and Condvar signal places), a requested count of 0 is interpreted as
//!   [`GoalPredicate::Empty`] (i.e. "no pending messages / no residual
//!   signal"), which is the semantically meaningful check. Any non-zero
//!   count falls back to [`GoalPredicate::Reachable`].
//! * Keys of the form `"{fn}.{sid}"` are mapped to the control place
//!   `cp_{fn}_{sid}` — useful for "thread X reached its return point".
//! * Keys starting with `cp_`, `rp_`, `wp_`, or `ra_` are treated as raw
//!   place IDs (advanced users / tooling-generated goals).
//! * Keys beginning with `var:` (e.g. `var:state`) attach to the CVN
//!   variable store even though they live in the marking map; this is
//!   tolerated for robustness but normally users should use
//!   `BusinessGoal.variables`.
//!
//! Unrecognised keys produce a warning-level diagnostic (collected into
//! the returned error vector but non-fatal: [`translate_goals`] still
//! returns the successfully-translated subset).

use std::collections::HashSet;

use concir::ast::{BaseType, ComplexBaseType, Program, Resource};
use cvn::analysis::goal::{GoalPredicate, GoalSpec};
use cvn::model::{ConcreteVal, PlaceId};
use serde_json::Value;

use super::context::{cp_id, rp_id};

/// Translate all business goals declared in `program` into CVN goal specs.
///
/// Returns `(specs, warnings)` where `warnings` lists goals that could not
/// be fully translated (e.g., unknown resource names).
pub fn translate_goals(program: &Program) -> (Vec<GoalSpec>, Vec<String>) {
    let mut specs = Vec::new();
    let mut warnings = Vec::new();

    let resource_by_name: std::collections::HashMap<&str, &Resource> =
        program.resources.iter().map(|r| (r.name.as_str(), r)).collect();
    let enum_variants: HashSet<String> = collect_enum_variants(program);
    let var_names: HashSet<&str> = program
        .resources
        .iter()
        .filter(|r| r.kind == "var")
        .map(|r| r.name.as_str())
        .collect();

    for goal in &program.goals {
        let mut predicates = Vec::new();

        for (key, count) in &goal.marking {
            match marking_predicate(key, *count, &resource_by_name) {
                Ok(pred) => predicates.push(pred),
                Err(msg) => warnings.push(format!(
                    "goal '{}': marking key '{}' — {}",
                    goal.id, key, msg
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
                    "goal '{}': variable '{}' — {}",
                    goal.id, var, msg
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
    count: u32,
    resources: &std::collections::HashMap<&str, &Resource>,
) -> Result<GoalPredicate, String> {
    // Raw place IDs are accepted as-is.
    if key.starts_with("cp_")
        || key.starts_with("rp_")
        || key.starts_with("wp_")
        || key.starts_with("ra_")
    {
        return Ok(reachability_predicate(PlaceId::new(key), count, false));
    }

    // "fn.sid" → control place.
    if let Some((fn_name, sid)) = key.split_once('.') {
        if !fn_name.is_empty() && !sid.is_empty() {
            return Ok(reachability_predicate(
                PlaceId::new(cp_id(fn_name, sid)),
                count,
                false,
            ));
        }
    }

    // Resource name lookup.
    if let Some(res) = resources.get(key) {
        let starts_empty = resource_starts_empty(res);
        return Ok(reachability_predicate(
            PlaceId::new(rp_id(key)),
            count,
            starts_empty,
        ));
    }

    Err("not a known resource, not a control-place reference, and not a raw place id".to_string())
}

/// Resources whose initial token count is 0.
///
/// For these places, a user goal of `count == 0` is meaningful as an
/// "empty / no residual" check. For all other resources (Mutex / RwLock /
/// Semaphore) the initial tokens are positive and `count == 0` just
/// degenerates to `tokens >= 0` which always holds, so we keep the
/// standard reachability semantics.
fn resource_starts_empty(res: &Resource) -> bool {
    matches!(
        (res.kind.as_str(), res.res_type.as_str()),
        ("sync", "Channel") | ("sync", "Condvar")
    )
}

fn reachability_predicate(
    place: PlaceId,
    count: u32,
    starts_empty: bool,
) -> GoalPredicate {
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

fn json_to_concrete(
    v: &Value,
    enum_variants: &HashSet<String>,
) -> Result<ConcreteVal, String> {
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
