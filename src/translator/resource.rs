use std::collections::HashSet;

use concir::ast::{BaseType, ComplexBaseType, Op, Program};
use unipn::model::ResourceType;

use super::context::{ResKind, TranslateContext, nw_var_name, rp_id};
use super::expr_parser::json_value_to_val_with_variants;
use crate::error::TranslateError;

/// Phase 1: Scan resources and produce resource places, initial marking, and initial V.
pub(crate) fn scan_resources(ctx: &mut TranslateContext, program: &Program) {
    let rwlock_n = compute_rwlock_n(program);
    ctx.rwlock_n = rwlock_n;

    for res in &program.resources {
        match (res.kind.as_str(), res.res_type.as_str()) {
            ("sync", "Mutex") => {
                ctx.resource_map
                    .insert(res.name.clone(), ResKind::Mutex);
                ctx.add_resource_place(&res.name, ResourceType::Mutex);
                ctx.set_initial_tokens(&rp_id(&res.name), 1);
            }
            ("sync", "RwLock") => {
                ctx.resource_map
                    .insert(res.name.clone(), ResKind::RwLock);
                ctx.add_resource_place(
                    &res.name,
                    ResourceType::RwLock {
                        max_readers: rwlock_n,
                    },
                );
                ctx.set_initial_tokens(&rp_id(&res.name), rwlock_n);
            }
            ("sync", "Condvar") => {
                ctx.resource_map
                    .insert(res.name.clone(), ResKind::Condvar);
                ctx.add_resource_place(&res.name, ResourceType::Condvar);
                // rp(cv) starts with 0 tokens (no pending notifications).
                ctx.add_variable(&nw_var_name(&res.name), unipn::Val::int(0));
            }
            ("sync", "Semaphore") => {
                let count = res.count.unwrap_or(1) as usize;
                ctx.resource_map
                    .insert(res.name.clone(), ResKind::Semaphore { count });
                ctx.add_resource_place(
                    &res.name,
                    ResourceType::Semaphore { count },
                );
                ctx.set_initial_tokens(&rp_id(&res.name), count);
            }
            ("sync", "Channel") => {
                ctx.resource_map
                    .insert(res.name.clone(), ResKind::Channel);
                ctx.add_resource_place(&res.name, ResourceType::Channel);
                // Channel starts with 0 tokens (no messages).
            }
            ("var", "Var") => {
                let enum_variants = extract_enum_variants(&res.base);
                for v in &enum_variants {
                    ctx.all_enum_variants.insert(v.clone());
                }
                ctx.resource_map.insert(
                    res.name.clone(),
                    ResKind::Var {
                        enum_variants: enum_variants.clone(),
                    },
                );
                // Add to variable store.
                if let Some(init) = &res.init {
                    let variant_set: HashSet<String> =
                        enum_variants.into_iter().collect();
                    let val = json_value_to_val_with_variants(init, &variant_set);
                    ctx.add_variable(&res.name, val);
                } else {
                    ctx.add_variable(&res.name, unipn::Val::Unknown);
                }
                set_bounded_domain(ctx, &res.name, &res.base);
            }
            ("var", "Atomic") => {
                let enum_variants = extract_enum_variants(&res.base);
                for v in &enum_variants {
                    ctx.all_enum_variants.insert(v.clone());
                }
                ctx.resource_map.insert(
                    res.name.clone(),
                    ResKind::Atomic {
                        enum_variants: enum_variants.clone(),
                    },
                );
                if let Some(init) = &res.init {
                    let variant_set: HashSet<String> =
                        enum_variants.into_iter().collect();
                    let val = json_value_to_val_with_variants(init, &variant_set);
                    ctx.add_variable(&res.name, val);
                } else {
                    ctx.add_variable(&res.name, unipn::Val::Unknown);
                }
                set_bounded_domain(ctx, &res.name, &res.base);
            }
            _ => {
                ctx.push_error(TranslateError::UnknownResourceType(format!(
                    "{}/{}",
                    res.kind, res.res_type
                )));
            }
        }
    }
}

/// Compute the RwLock N value: number of distinct thread contexts.
///
/// N = (unique function names referenced by spawn/spawn_async) + 1 (for entry).
fn compute_rwlock_n(program: &Program) -> usize {
    let mut spawned: HashSet<&str> = HashSet::new();
    for func in &program.functions {
        for stmt in &func.body {
            match &stmt.op {
                Op::Spawn(f) | Op::SpawnAsync(f) => {
                    spawned.insert(f.as_str());
                }
                _ => {}
            }
        }
    }
    (spawned.len() as usize) + 1
}

/// Extract enum variant names from a ConcIR `BaseType`, if it is an Enum.
fn extract_enum_variants(base: &Option<BaseType>) -> Vec<String> {    match base {
        Some(BaseType::Complex(ComplexBaseType::Enum(variants))) => variants.clone(),
        _ => Vec::new(),
    }
}

/// Declare the Int value domain of a variable whose base is a bounded Int.
fn set_bounded_domain(ctx: &mut TranslateContext, var_name: &str, base: &Option<BaseType>) {
    if let Some(BaseType::Complex(ComplexBaseType::BoundedInt { lo, hi })) = base {
        ctx.set_variable_domain(var_name, *lo, *hi);
    }
}
