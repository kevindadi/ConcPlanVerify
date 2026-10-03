//! Supported-subset channel construction and endpoint tracing.
//!
//! Identity is decided elsewhere. This module records, for a direct
//! `let (tx, rx) = channel()` / `sync_channel(n)` and for sends or recvs on
//! names reached by a simple alias or one call-argument pass:
//! unbounded, rendezvous (capacity 0), bounded(n), or unknown.
//!
//! A variable name is not capacity evidence. An unused channel is not evidence
//! for a different endpoint. Unsupported control flow makes the affected use
//! unknown. `Barrier::new` and a traced `wait` are recorded as uncovered sync;
//! they are not given a CIR semantics.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use syn::spanned::Spanned;
use syn::{Block, Expr, File, Item, Local, Pat, Stmt};

const BARRIER: &str = "__barrier__";

#[derive(Debug, Clone, Serialize)]
pub struct Construction {
    pub id: String,
    pub function: String,
    pub site: String,
    pub constructor: String,
    pub capacity_class: String,
    pub capacity: Option<i64>,
    pub tx: String,
    pub rx: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EndpointLink {
    pub function: String,
    pub name: String,
    pub construction_id: Option<String>,
    pub status: String,
    pub evidence: String,
    pub site: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UncoveredSync {
    pub kind: String,
    pub form: String,
    pub site: String,
    pub function: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SemaphoreInit {
    pub id: String,
    /// Name of the `let` that contains `Semaphore::new`, not a later alias.
    pub binding: String,
    /// Byte offset of the `new` identifier, the same site the instrument records.
    pub site: String,
    pub initial: Option<i64>,
    /// `literal_initial`, `unknown_initial`, `flow_unknown`, or `unresolved_use`.
    pub evidence: String,
    /// True only when a straight-line acquire/release uses this construction.
    pub used: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Facts {
    pub constructions: Vec<Construction>,
    pub endpoints: Vec<EndpointLink>,
    pub uncovered_sync: Vec<UncoveredSync>,
    #[serde(default)]
    pub semaphores: Vec<SemaphoreInit>,
}

#[derive(Clone, PartialEq, Eq)]
enum Link {
    Traced(String),
    /// A function parameter, resolved from the call argument after the walk.
    Param,
    Unknown,
}

struct Call {
    callee: String,
    args: Vec<Link>,
}

struct Wait {
    name: String,
    site: String,
    link: Link,
}

struct UseSite {
    name: String,
    site: String,
    link: Link,
}

struct FnFact {
    params: Vec<String>,
    /// Innermost scope last. A nested `let` shadows; leaving the block restores
    /// the outer binding. Assignment updates the binding that is in scope.
    env: Vec<BTreeMap<String, Link>>,
    calls: Vec<Call>,
    uses: Vec<UseSite>,
    waits: Vec<Wait>,
    flow_unknown: bool,
}

pub fn analyze(src: &str) -> Result<Facts, String> {
    let file: File = syn::parse_file(src).map_err(|e| format!("parse error: {e}"))?;
    let starts = line_starts(src);
    let mut fns: BTreeMap<String, FnFact> = BTreeMap::new();
    let mut constructions = Vec::new();
    let mut uncovered = Vec::new();
    let mut next_id = 0usize;

    for item in &file.items {
        let Item::Fn(func) = item else { continue };
        let name = func.sig.ident.to_string();
        let params = func
            .sig
            .inputs
            .iter()
            .filter_map(|arg| match arg {
                syn::FnArg::Typed(t) => ident_pat(&t.pat),
                syn::FnArg::Receiver(_) => None,
            })
            .collect::<Vec<_>>();
        let mut fact = FnFact {
            params: params.clone(),
            env: vec![BTreeMap::new()],
            calls: Vec::new(),
            uses: Vec::new(),
            waits: Vec::new(),
            flow_unknown: false,
        };
        for param in &params {
            fact.env[0].insert(param.clone(), Link::Param);
        }
        walk_block(
            &func.block,
            &name,
            &starts,
            &mut fact,
            &mut constructions,
            &mut uncovered,
            &mut next_id,
        );
        fns.insert(name, fact);
    }

    let known: BTreeSet<String> = fns.keys().cloned().collect();
    let mut param_link: BTreeMap<(String, String), Link> = BTreeMap::new();
    for fact in fns.values() {
        for call in &fact.calls {
            if !known.contains(&call.callee) {
                continue;
            }
            let Some(callee) = fns.get(&call.callee) else { continue };
            if fact.flow_unknown || callee.flow_unknown || call.args.len() != callee.params.len() {
                for param in &callee.params {
                    param_link.insert((call.callee.clone(), param.clone()), Link::Unknown);
                }
                continue;
            }
            for (param, arg) in callee.params.iter().zip(call.args.iter()) {
                let key = (call.callee.clone(), param.clone());
                match param_link.get(&key) {
                    None => {
                        param_link.insert(key, arg.clone());
                    }
                    Some(prev) if same_link(prev, arg) => {}
                    Some(_) => {
                        param_link.insert(key, Link::Unknown);
                    }
                }
            }
        }
    }

    let mut endpoints = Vec::new();
    for (function, fact) in &fns {
        let mut by_name: BTreeMap<String, Vec<&UseSite>> = BTreeMap::new();
        for site in &fact.uses {
            by_name.entry(site.name.clone()).or_default().push(site);
        }
        for (name, sites) in by_name {
            let resolved: Vec<Link> = sites
                .iter()
                .map(|site| resolve_link(fact, function, &site.name, &site.link, &param_link))
                .collect();
            let (construction_id, status, evidence) = if fact.flow_unknown
                || resolved.iter().any(|link| matches!(link, Link::Unknown | Link::Param))
                || resolved.is_empty()
            {
                (None, "unknown".to_string(), "unresolved_endpoint".to_string())
            } else {
                let ids: BTreeSet<String> = resolved
                    .iter()
                    .filter_map(|link| match link {
                        Link::Traced(id) if id != BARRIER && constructions.iter().any(|c| &c.id == id) => {
                            Some(id.clone())
                        }
                        _ => None,
                    })
                    .collect();
                if ids.len() == 1 {
                    (
                        ids.iter().next().cloned(),
                        "traced".to_string(),
                        if fact.params.iter().any(|p| p == &name) {
                            "param_from_call".to_string()
                        } else {
                            "local_binding".to_string()
                        },
                    )
                } else {
                    (None, "unknown".to_string(), "rebinding".to_string())
                }
            };
            endpoints.push(EndpointLink {
                function: function.clone(),
                name,
                construction_id,
                status,
                evidence,
                site: sites.first().map(|site| site.site.clone()),
            });
        }
        for wait in &fact.waits {
            let link = resolve_link(fact, function, &wait.name, &wait.link, &param_link);
            if matches!(link, Link::Traced(id) if id == BARRIER) {
                push_unique(
                    &mut uncovered,
                    UncoveredSync {
                        kind: "Barrier".to_string(),
                        form: "wait".to_string(),
                        site: wait.site.clone(),
                        function: function.clone(),
                        reason: "Barrier::wait is not a modeled CIR event".to_string(),
                    },
                );
            }
        }
    }

    let semaphores = collect_semaphores(&file, &starts);
    Ok(Facts { constructions, endpoints, uncovered_sync: uncovered, semaphores })
}

#[derive(Clone)]
enum SemLink {
    Id(String),
    Param,
    /// A zero-argument closure that returns a tuple of clones or paths.
    Tuple(Vec<SemLink>),
    Unknown,
}

struct SemCall {
    callee: String,
    args: Vec<SemLink>,
}

struct SemFn {
    params: Vec<String>,
    env: Vec<BTreeMap<String, SemLink>>,
    calls: Vec<SemCall>,
    param_used: BTreeSet<String>,
    flow_unknown: bool,
    unresolved_use: bool,
    control_depth: usize,
}

fn collect_semaphores(file: &File, starts: &[usize]) -> Vec<SemaphoreInit> {
    let mut fns: BTreeMap<String, SemFn> = BTreeMap::new();
    let mut inits = Vec::new();
    let mut next_id = 0usize;
    for item in &file.items {
        let Item::Fn(func) = item else { continue };
        let name = func.sig.ident.to_string();
        let params: Vec<String> = func
            .sig
            .inputs
            .iter()
            .filter_map(|arg| match arg {
                syn::FnArg::Typed(pat) => ident_pat(&pat.pat),
                syn::FnArg::Receiver(_) => None,
            })
            .collect();
        let mut env0 = BTreeMap::new();
        for param in &params {
            env0.insert(param.clone(), SemLink::Param);
        }
        let start = inits.len();
        let mut state = SemFn {
            params,
            env: vec![env0],
            calls: Vec::new(),
            param_used: BTreeSet::new(),
            flow_unknown: false,
            unresolved_use: false,
            control_depth: 0,
        };
        walk_sem_block(&func.block, starts, &mut state, &mut inits, &mut next_id);
        if state.flow_unknown {
            for init in &mut inits[start..] {
                init.used = false;
                init.evidence = "flow_unknown".into();
            }
        } else if state.unresolved_use {
            for init in &mut inits[start..] {
                init.used = false;
                init.evidence = "unresolved_use".into();
            }
        }
        fns.insert(name, state);
    }
    resolve_sem_calls(&fns, &mut inits);
    inits
}

fn resolve_sem_calls(fns: &BTreeMap<String, SemFn>, inits: &mut [SemaphoreInit]) {
    let mut bad = false;
    let mut marks = Vec::new();
    for func in fns.values() {
        if func.flow_unknown || func.unresolved_use {
            continue;
        }
        for call in &func.calls {
            let Some(callee) = fns.get(&call.callee) else { continue };
            if callee.flow_unknown || callee.unresolved_use {
                bad = true;
                continue;
            }
            if callee.params.len() != call.args.len()
                && callee.params.iter().any(|p| callee.param_used.contains(p))
            {
                bad = true;
                continue;
            }
            for (param, arg) in callee.params.iter().zip(call.args.iter()) {
                if !callee.param_used.contains(param) {
                    continue;
                }
                match arg {
                    SemLink::Id(id) => marks.push(id.clone()),
                    SemLink::Param | SemLink::Unknown | SemLink::Tuple(_) => bad = true,
                }
            }
        }
    }
    if bad {
        for init in inits.iter_mut() {
            init.used = false;
            if init.evidence == "literal_initial" || init.evidence == "unknown_initial" {
                init.evidence = "unresolved_use".into();
            }
        }
        return;
    }
    for id in marks {
        if let Some(init) = inits.iter_mut().find(|item| item.id == id) {
            if init.evidence == "literal_initial" || init.evidence == "unknown_initial" {
                init.used = true;
            }
        }
    }
}

impl SemFn {
    fn lookup(&self, name: &str) -> Option<SemLink> {
        for scope in self.env.iter().rev() {
            if let Some(link) = scope.get(name) {
                return Some(link.clone());
            }
        }
        None
    }

    fn insert(&mut self, name: String, link: SemLink) {
        if let Some(scope) = self.env.last_mut() {
            scope.insert(name, link);
        }
    }

    fn assign(&mut self, name: &str, link: SemLink) {
        for scope in self.env.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), link);
                return;
            }
        }
        if let Some(scope) = self.env.last_mut() {
            scope.insert(name.to_string(), SemLink::Unknown);
        }
    }
}

fn walk_sem_block(
    block: &Block,
    starts: &[usize],
    state: &mut SemFn,
    inits: &mut Vec<SemaphoreInit>,
    next_id: &mut usize,
) {
    for stmt in &block.stmts {
        walk_sem_stmt(stmt, starts, state, inits, next_id);
    }
}

fn walk_sem_stmt(
    stmt: &Stmt,
    starts: &[usize],
    state: &mut SemFn,
    inits: &mut Vec<SemaphoreInit>,
    next_id: &mut usize,
) {
    match stmt {
        Stmt::Local(local) => {
            if let Some(link) = closure_tuple_link(local, starts, state, inits, next_id) {
                if let Some(name) = ident_pat(&local.pat) {
                    state.insert(name, link);
                }
                return;
            }
            if let Some(names) = tuple_binding_names(&local.pat) {
                let links = local.init.as_ref().and_then(|init| zero_arg_call(&init.expr))
                    .and_then(|name| match state.lookup(&name) {
                        Some(SemLink::Tuple(links)) => Some(links),
                        _ => None,
                    });
                if let Some(links) = links {
                    if links.len() == names.len() {
                        for (name, link) in names.into_iter().zip(links) {
                            state.insert(name, link);
                        }
                        return;
                    }
                }
                for name in names {
                    state.insert(name, SemLink::Unknown);
                }
                return;
            }
            let name = ident_pat(&local.pat);
            let link = match &local.init {
                Some(init) => take_link(&init.expr, name.as_deref(), starts, state, inits, next_id),
                None => SemLink::Unknown,
            };
            if let Some(name) = name {
                state.insert(name, link);
            } else if !matches!(local.pat, Pat::Wild(_)) {
                // A wildcard discards the value and introduces no alias. Its
                // initializer was still walked above, including semaphore uses.
                // Unsupported binding patterns remain conservative.
                state.flow_unknown = true;
            }
        }
        Stmt::Expr(expr, _) => {
            if let Expr::Assign(assign) = expr {
                let link = take_link(&assign.right, path_ident(&assign.left).as_deref(), starts, state, inits, next_id);
                if let Some(name) = path_ident(&assign.left) {
                    state.assign(&name, link);
                }
            } else {
                walk_sem_expr(expr, starts, state, inits, next_id);
            }
        }
        Stmt::Item(_) => state.flow_unknown = true,
        Stmt::Macro(_) => {}
    }
}

fn walk_sem_expr(
    expr: &Expr,
    starts: &[usize],
    state: &mut SemFn,
    inits: &mut Vec<SemaphoreInit>,
    next_id: &mut usize,
) {
    match expr {
        Expr::Block(inner) => {
            state.env.push(BTreeMap::new());
            walk_sem_block(&inner.block, starts, state, inits, next_id);
            state.env.pop();
        }
        Expr::Closure(inner) => walk_sem_expr(&inner.body, starts, state, inits, next_id),
        Expr::If(inner) => {
            state.control_depth += 1;
            walk_sem_expr(&inner.cond, starts, state, inits, next_id);
            walk_sem_block(&inner.then_branch, starts, state, inits, next_id);
            if let Some((_, else_expr)) = &inner.else_branch {
                walk_sem_expr(else_expr, starts, state, inits, next_id);
            }
            state.control_depth -= 1;
        }
        Expr::Match(inner) => {
            state.control_depth += 1;
            walk_sem_expr(&inner.expr, starts, state, inits, next_id);
            for arm in &inner.arms {
                walk_sem_expr(&arm.body, starts, state, inits, next_id);
            }
            state.control_depth -= 1;
        }
        Expr::While(inner) => {
            state.control_depth += 1;
            walk_sem_expr(&inner.cond, starts, state, inits, next_id);
            walk_sem_block(&inner.body, starts, state, inits, next_id);
            state.control_depth -= 1;
        }
        Expr::Loop(inner) => {
            state.control_depth += 1;
            walk_sem_block(&inner.body, starts, state, inits, next_id);
            state.control_depth -= 1;
        }
        Expr::ForLoop(inner) => {
            state.control_depth += 1;
            walk_sem_expr(&inner.expr, starts, state, inits, next_id);
            walk_sem_block(&inner.body, starts, state, inits, next_id);
            state.control_depth -= 1;
        }
        Expr::Try(inner) => {
            state.control_depth += 1;
            walk_sem_expr(&inner.expr, starts, state, inits, next_id);
            state.control_depth -= 1;
        }
        Expr::MethodCall(call) => {
            note_sem_method(&call.method.to_string(), &call.receiver, state, inits);
            walk_sem_expr(&call.receiver, starts, state, inits, next_id);
            for arg in &call.args {
                walk_sem_expr(arg, starts, state, inits, next_id);
            }
        }
        Expr::Call(call) => {
            if let Some(name) = user_call(&call.func) {
                let args = call
                    .args
                    .iter()
                    .map(|arg| take_link(arg, None, starts, state, inits, next_id))
                    .collect();
                state.calls.push(SemCall { callee: name, args });
            } else {
                walk_sem_expr(&call.func, starts, state, inits, next_id);
                for arg in &call.args {
                    walk_sem_expr(arg, starts, state, inits, next_id);
                }
            }
        }
        Expr::Assign(assign) => {
            let link = take_link(&assign.right, path_ident(&assign.left).as_deref(), starts, state, inits, next_id);
            if let Some(name) = path_ident(&assign.left) {
                state.assign(&name, link);
            }
        }
        Expr::Reference(inner) => walk_sem_expr(&inner.expr, starts, state, inits, next_id),
        Expr::Field(inner) => walk_sem_expr(&inner.base, starts, state, inits, next_id),
        Expr::Paren(inner) => walk_sem_expr(&inner.expr, starts, state, inits, next_id),
        Expr::Unary(inner) => walk_sem_expr(&inner.expr, starts, state, inits, next_id),
        Expr::Binary(inner) => {
            walk_sem_expr(&inner.left, starts, state, inits, next_id);
            walk_sem_expr(&inner.right, starts, state, inits, next_id);
        }
        Expr::Tuple(inner) => {
            for elem in &inner.elems {
                walk_sem_expr(elem, starts, state, inits, next_id);
            }
        }
        Expr::Array(inner) => {
            for elem in &inner.elems {
                walk_sem_expr(elem, starts, state, inits, next_id);
            }
        }
        Expr::Repeat(inner) => walk_sem_expr(&inner.expr, starts, state, inits, next_id),
        Expr::Cast(inner) => walk_sem_expr(&inner.expr, starts, state, inits, next_id),
        Expr::Let(inner) => walk_sem_expr(&inner.expr, starts, state, inits, next_id),
        Expr::Return(inner) => {
            if let Some(value) = &inner.expr {
                walk_sem_expr(value, starts, state, inits, next_id);
            }
        }
        Expr::Async(_) | Expr::TryBlock(_) | Expr::Yield(_) | Expr::Await(_) => state.flow_unknown = true,
        _ => {}
    }
}

fn note_sem_method(method: &str, receiver: &Expr, state: &mut SemFn, inits: &mut [SemaphoreInit]) {
    if !matches!(method, "acquire" | "try_acquire" | "acquire_count" | "release_count") {
        return;
    }
    let Some(name) = path_ident(receiver) else {
        state.unresolved_use = true;
        return;
    };
    if state.control_depth > 0 {
        if state.lookup(&name).is_some() {
            state.unresolved_use = true;
        }
        return;
    }
    match state.lookup(&name) {
        Some(SemLink::Id(id)) => {
            if let Some(init) = inits.iter_mut().find(|item| item.id == id) {
                if init.evidence == "literal_initial" || init.evidence == "unknown_initial" {
                    init.used = true;
                }
            }
        }
        Some(SemLink::Param) => {
            state.param_used.insert(name);
        }
        Some(SemLink::Unknown) | Some(SemLink::Tuple(_)) => state.unresolved_use = true,
        None => {}
    }
}

fn take_link(
    expr: &Expr,
    binding: Option<&str>,
    starts: &[usize],
    state: &mut SemFn,
    inits: &mut Vec<SemaphoreInit>,
    next_id: &mut usize,
) -> SemLink {
    if let Some(link) = sem_value(expr, binding, starts, state, inits, next_id) {
        return link;
    }
    walk_sem_expr(expr, starts, state, inits, next_id);
    SemLink::Unknown
}

fn sem_value(
    expr: &Expr,
    binding: Option<&str>,
    starts: &[usize],
    state: &mut SemFn,
    inits: &mut Vec<SemaphoreInit>,
    next_id: &mut usize,
) -> Option<SemLink> {
    if let Some(inner) = clone_target(expr) {
        return Some(sem_value(inner, None, starts, state, inits, next_id).unwrap_or(SemLink::Unknown));
    }
    // Semaphore::new returns Arc<Semaphore>. Only propagate method-clone from
    // a known constructor identity, never from an untyped/unknown receiver.
    if let Expr::MethodCall(call) = expr {
        if call.method == "clone" && call.args.is_empty() {
            if let Some(link @ SemLink::Id(_)) =
                sem_value(&call.receiver, None, starts, state, inits, next_id)
            {
                return Some(link);
            }
        }
    }
    if let Expr::Reference(inner) = expr {
        return sem_value(&inner.expr, binding, starts, state, inits, next_id);
    }
    if let Expr::Block(inner) = expr {
        if let Some(tail) = block_tail(&inner.block) {
            state.env.push(BTreeMap::new());
            for stmt in &inner.block.stmts[..inner.block.stmts.len().saturating_sub(1)] {
                walk_sem_stmt(stmt, starts, state, inits, next_id);
            }
            let link = match sem_value(tail, binding, starts, state, inits, next_id) {
                Some(link) => link,
                None => {
                    walk_sem_expr(tail, starts, state, inits, next_id);
                    SemLink::Unknown
                }
            };
            state.env.pop();
            return Some(link);
        }
    }
    if let Some((initial, evidence, site)) = semaphore_init_from(expr, starts) {
        let Some(binding) = binding else {
            state.unresolved_use = true;
            return Some(SemLink::Unknown);
        };
        let id = format!("sem{next_id}");
        *next_id += 1;
        inits.push(SemaphoreInit {
            id: id.clone(),
            binding: binding.to_string(),
            site,
            initial,
            evidence,
            used: false,
        });
        return Some(SemLink::Id(id));
    }
    if let Some(name) = path_ident(expr) {
        return Some(state.lookup(&name).unwrap_or(SemLink::Unknown));
    }
    None
}

fn block_tail(block: &Block) -> Option<&Expr> {
    match block.stmts.last()? {
        Stmt::Expr(expr, None) => Some(expr),
        _ => None,
    }
}

fn clone_target(expr: &Expr) -> Option<&Expr> {
    let Expr::Call(call) = expr else { return None };
    if path_last(&call.func).as_deref() != Some("clone") || !path_contains(&call.func, "Arc") {
        return None;
    }
    let arg = call.args.first()?;
    Some(match arg {
        Expr::Reference(inner) => &inner.expr,
        other => other,
    })
}

fn closure_tuple_link(
    local: &Local,
    starts: &[usize],
    state: &mut SemFn,
    inits: &mut Vec<SemaphoreInit>,
    next_id: &mut usize,
) -> Option<SemLink> {
    let init = local.init.as_ref()?;
    let Expr::Closure(closure) = &*init.expr else { return None };
    if !closure.inputs.is_empty() {
        return None;
    }
    let Expr::Tuple(tuple) = &*closure.body else { return None };
    let links = tuple
        .elems
        .iter()
        .map(|elem| sem_value(elem, None, starts, state, inits, next_id).unwrap_or(SemLink::Unknown))
        .collect();
    Some(SemLink::Tuple(links))
}

fn tuple_binding_names(pat: &Pat) -> Option<Vec<String>> {
    let Pat::Tuple(tuple) = pat else { return None };
    tuple.elems.iter().map(ident_pat).collect()
}

fn zero_arg_call(expr: &Expr) -> Option<String> {
    let Expr::Call(call) = expr else { return None };
    if !call.args.is_empty() {
        return None;
    }
    user_call(&call.func)
}

fn user_call(func: &Expr) -> Option<String> {
    let Expr::Path(path) = func else { return None };
    if path.path.segments.len() != 1 {
        return None;
    }
    Some(path.path.segments.last()?.ident.to_string())
}

fn semaphore_init_from(expr: &Expr, starts: &[usize]) -> Option<(Option<i64>, String, String)> {
    let Expr::Call(call) = expr else { return None };
    if !path_contains(&call.func, "Semaphore") {
        return None;
    }
    let last = path_last(&call.func)?;
    if last != "new" && last != "new_named" {
        return None;
    }
    let Expr::Path(path) = &*call.func else { return None };
    let ident = &path.path.segments.last()?.ident;
    let site = offset(starts, ident.span().start());
    let count_expr = if last == "new_named" { call.args.get(1)? } else { call.args.first()? };
    let initial = int_literal(count_expr);
    let evidence = if initial.is_some() { "literal_initial" } else { "unknown_initial" };
    Some((initial, evidence.to_string(), site))
}

fn resolve_link(
    fact: &FnFact,
    function: &str,
    name: &str,
    link: &Link,
    param_link: &BTreeMap<(String, String), Link>,
) -> Link {
    if fact.flow_unknown {
        return Link::Unknown;
    }
    match link {
        Link::Param => param_link
            .get(&(function.to_string(), name.to_string()))
            .cloned()
            .unwrap_or(Link::Unknown),
        other => other.clone(),
    }
}

impl FnFact {
    fn lookup(&self, name: &str) -> Option<Link> {
        for scope in self.env.iter().rev() {
            if let Some(link) = scope.get(name) {
                return Some(link.clone());
            }
        }
        None
    }

    fn insert(&mut self, name: String, link: Link) {
        if let Some(scope) = self.env.last_mut() {
            scope.insert(name, link);
        }
    }

    /// Is this name currently bound to a traced channel endpoint?
    fn is_traced(&self, name: &str) -> bool {
        matches!(self.lookup(name), Some(Link::Traced(_)))
    }

    /// Update the in-scope binding. An unknown right-hand side replaces the old
    /// construction; it does not keep it.
    fn assign(&mut self, name: &str, link: Link) {
        for scope in self.env.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), link);
                return;
            }
        }
        if let Some(scope) = self.env.last_mut() {
            scope.insert(name.to_string(), Link::Unknown);
        }
    }
}

fn same_link(a: &Link, b: &Link) -> bool {
    match (a, b) {
        (Link::Traced(x), Link::Traced(y)) => x == y,
        (Link::Unknown, Link::Unknown) => true,
        _ => false,
    }
}

fn walk_block(
    block: &Block,
    function: &str,
    starts: &[usize],
    fact: &mut FnFact,
    constructions: &mut Vec<Construction>,
    uncovered: &mut Vec<UncoveredSync>,
    next_id: &mut usize,
) {
    for stmt in &block.stmts {
        walk_stmt(stmt, function, starts, fact, constructions, uncovered, next_id);
    }
}

fn walk_stmt(
    stmt: &Stmt,
    function: &str,
    starts: &[usize],
    fact: &mut FnFact,
    constructions: &mut Vec<Construction>,
    uncovered: &mut Vec<UncoveredSync>,
    next_id: &mut usize,
) {
    match stmt {
        Stmt::Local(local) => {
            handle_local(local, function, starts, fact, constructions, uncovered, next_id);
        }
        Stmt::Expr(expr, _) => {
            if let Expr::Block(inner) = expr {
                fact.env.push(BTreeMap::new());
                walk_block(&inner.block, function, starts, fact, constructions, uncovered, next_id);
                fact.env.pop();
            } else {
                walk_expr(expr, function, starts, fact, uncovered);
            }
        }
        Stmt::Item(_) => fact.flow_unknown = true,
        Stmt::Macro(_) => {}
    }
}

fn handle_local(
    local: &Local,
    function: &str,
    starts: &[usize],
    fact: &mut FnFact,
    constructions: &mut Vec<Construction>,
    uncovered: &mut Vec<UncoveredSync>,
    next_id: &mut usize,
) {
    let Some(init) = &local.init else { return };
    walk_expr(&init.expr, function, starts, fact, uncovered);
    if let Some(site_expr) = find_barrier_new(&init.expr) {
        push_unique(
            uncovered,
            UncoveredSync {
                kind: "Barrier".to_string(),
                form: "Barrier::new".to_string(),
                site: offset(starts, site_expr.span().start()),
                function: function.to_string(),
                reason: "Barrier is not part of the modeled CIR event projection".to_string(),
            },
        );
    }
    if let Some((tx, rx)) = tuple_names(&local.pat) {
        if let Some(ctor) = channel_ctor(&init.expr) {
            let id = format!("ctor{next_id}");
            *next_id += 1;
            constructions.push(Construction {
                id: id.clone(),
                function: function.to_string(),
                site: offset(starts, init.expr.span().start()),
                constructor: ctor.constructor,
                capacity_class: ctor.class.clone(),
                capacity: ctor.capacity,
                tx: tx.clone(),
                rx: rx.clone(),
                evidence: "let_tuple".to_string(),
            });
            let link = Link::Traced(id);
            fact.insert(tx, link.clone());
            fact.insert(rx, link);
            return;
        }
        // A new tuple binding always hides older names, even when the
        // initializer is not a supported constructor.
        fact.insert(tx, Link::Unknown);
        fact.insert(rx, Link::Unknown);
        return;
    }
    if let Some(name) = ident_pat(&local.pat) {
        if is_barrier_expr(&init.expr) || find_barrier_new(&init.expr).is_some() {
            fact.insert(name, Link::Traced(BARRIER.to_string()));
            return;
        }
        let link = value_link(
            &init.expr, function, starts, fact, constructions, uncovered, next_id,
        );
        fact.insert(name, link);
    }
}

fn value_link(
    expr: &Expr,
    function: &str,
    starts: &[usize],
    fact: &mut FnFact,
    constructions: &mut Vec<Construction>,
    uncovered: &mut Vec<UncoveredSync>,
    next_id: &mut usize,
) -> Link {
    if let Some(name) = path_ident(expr) {
        return fact.lookup(&name).unwrap_or(Link::Unknown);
    }
    if let Some(src) = arc_clone_target(expr) {
        return fact.lookup(&src).unwrap_or(Link::Unknown);
    }
    // Cloning a known std channel endpoint preserves its constructor. An
    // unknown or shadowed receiver must not inherit an earlier capacity.
    if let Expr::MethodCall(call) = expr {
        if call.method == "clone" && call.args.is_empty() {
            if let Some(src) = path_ident(&call.receiver) {
                if let Some(link @ Link::Traced(_)) = fact.lookup(&src) {
                    return link;
                }
            }
        }
    }
    if let Expr::Paren(inner) = expr {
        return value_link(&inner.expr, function, starts, fact, constructions, uncovered, next_id);
    }
    if let Expr::Block(block) = expr {
        fact.env.push(BTreeMap::new());
        let stmts = &block.block.stmts;
        let link = if let Some((last, rest)) = stmts.split_last() {
            for stmt in rest {
                walk_stmt(stmt, function, starts, fact, constructions, uncovered, next_id);
            }
            match last {
                Stmt::Expr(tail, _) => value_link(
                    tail, function, starts, fact, constructions, uncovered, next_id,
                ),
                other => {
                    walk_stmt(other, function, starts, fact, constructions, uncovered, next_id);
                    Link::Unknown
                }
            }
        } else {
            Link::Unknown
        };
        fact.env.pop();
        return link;
    }
    Link::Unknown
}

struct Ctor {
    constructor: String,
    class: String,
    capacity: Option<i64>,
}

fn channel_ctor(expr: &Expr) -> Option<Ctor> {
    let Expr::Call(call) = expr else { return None };
    let name = path_last(&call.func)?;
    if name != "channel" && name != "sync_channel" {
        return None;
    }
    if name == "channel" {
        let class = if call.args.is_empty() { "unbounded" } else { "unknown" };
        return Some(Ctor {
            constructor: "channel".into(),
            class: class.into(),
            capacity: None,
        });
    }
    let (class, capacity) = match call.args.len() {
        1 => match int_literal(&call.args[0]) {
            Some(0) => ("rendezvous", Some(0)),
            Some(n) if n > 0 => ("bounded", Some(n)),
            _ => ("unknown", None),
        },
        _ => ("unknown", None),
    };
    Some(Ctor { constructor: "sync_channel".into(), class: class.into(), capacity })
}

fn walk_expr(expr: &Expr, function: &str, starts: &[usize], fact: &mut FnFact, uncovered: &mut Vec<UncoveredSync>) {
    match expr {
        Expr::If(_) | Expr::Match(_) | Expr::While(_) | Expr::Loop(_) | Expr::ForLoop(_) => {
            // Only control flow that actually constructs, sends/recvs on, or
            // rebinds a channel endpoint makes that identity conditional. An
            // unrelated sequential branch or computation must not erase the
            // endpoint identity of the whole function.
            if expr_affects_channels(expr, fact) {
                fact.flow_unknown = true;
            }
            if let Some(site_expr) = find_barrier_new(expr) {
                push_unique(
                    uncovered,
                    UncoveredSync {
                        kind: "Barrier".into(),
                        form: "Barrier::new".into(),
                        site: offset(starts, site_expr.span().start()),
                        function: function.into(),
                        reason: "Barrier is not part of the modeled CIR event projection".into(),
                    },
                );
            }
        }
        Expr::Block(_) => {
            // Statement blocks are scoped by walk_block. An expression block
            // reached here is still its own scope.
        }
        Expr::Call(call) => {
            if let Some(name) = path_last(&call.func) {
                if name != "channel" && name != "sync_channel" && name != "spawn" && name != "new" && name != "clone"
                {
                    fact.calls.push(Call {
                        callee: name,
                        args: call.args.iter().map(|arg| arg_link(arg, fact)).collect(),
                    });
                }
            }
            for arg in &call.args {
                walk_expr(arg, function, starts, fact, uncovered);
            }
        }
        Expr::MethodCall(call) => {
            let method = call.method.to_string();
            if matches!(method.as_str(), "send" | "recv" | "try_send" | "try_recv") {
                if let Some(name) = path_ident(&call.receiver) {
                    let link = fact.lookup(&name).unwrap_or(Link::Unknown);
                    fact.uses.push(UseSite {
                        name,
                        site: offset(starts, call.span().start()),
                        link,
                    });
                } else {
                    fact.flow_unknown = true;
                }
            }
            if method == "wait" {
                if let Some(name) = path_ident(&call.receiver) {
                    let link = fact.lookup(&name).unwrap_or(Link::Unknown);
                    fact.waits.push(Wait {
                        name,
                        site: offset(starts, call.span().start()),
                        link,
                    });
                }
            }
            walk_expr(&call.receiver, function, starts, fact, uncovered);
            for arg in &call.args {
                walk_expr(arg, function, starts, fact, uncovered);
            }
        }
        Expr::Closure(c) => walk_closure_body(&c.body, function, starts, fact, uncovered),
        Expr::Reference(r) => walk_expr(&r.expr, function, starts, fact, uncovered),
        Expr::Paren(p) => walk_expr(&p.expr, function, starts, fact, uncovered),
        Expr::Assign(a) => {
            walk_expr(&a.right, function, starts, fact, uncovered);
            if let Some(name) = path_ident(&a.left) {
                let link = path_ident(&a.right)
                    .and_then(|src| fact.lookup(&src))
                    .unwrap_or(Link::Unknown);
                fact.assign(&name, link);
            } else {
                fact.flow_unknown = true;
            }
        }
        Expr::Field(f) => walk_expr(&f.base, function, starts, fact, uncovered),
        Expr::Tuple(t) => {
            for elem in &t.elems {
                walk_expr(elem, function, starts, fact, uncovered);
            }
        }
        _ => {}
    }
}

fn walk_closure_body(
    expr: &Expr,
    function: &str,
    starts: &[usize],
    fact: &mut FnFact,
    uncovered: &mut Vec<UncoveredSync>,
) {
    if let Expr::Block(block) = expr {
        fact.env.push(BTreeMap::new());
        for stmt in &block.block.stmts {
            match stmt {
                Stmt::Expr(expr, _) => walk_closure_body(expr, function, starts, fact, uncovered),
                // A local that merely moves or discards an endpoint does not
                // change any construction identity. Only a binding that
                // shadows a traced endpoint, constructs a channel, or sends /
                // recvs inside its initializer makes the function's channel
                // identity conditional.
                Stmt::Local(local) => {
                    if local_affects_channels(local, fact) {
                        fact.flow_unknown = true;
                    } else if let Some(init) = &local.init {
                        // No channel identity is at stake; still record any
                        // user call so cross-function parameter links work.
                        walk_expr(&init.expr, function, starts, fact, uncovered);
                    }
                }
                Stmt::Item(_) => fact.flow_unknown = true,
                Stmt::Macro(_) => {}
            }
        }
        fact.env.pop();
    } else {
        walk_expr(expr, function, starts, fact, uncovered);
    }
}

/// Does this expression construct, send/recv on, or rebind a channel endpoint?
/// A bare path reference (a move or borrow) is not identity-changing and does
/// not count; the caller stays conservative whenever the answer is yes.
fn expr_affects_channels(expr: &Expr, fact: &FnFact) -> bool {
    match expr {
        Expr::Call(call) => {
            matches!(
                path_last(&call.func).as_deref(),
                Some("channel") | Some("sync_channel")
            ) || call.args.iter().any(|a| expr_affects_channels(a, fact))
        }
        Expr::MethodCall(call) => {
            matches!(
                call.method.to_string().as_str(),
                "send" | "recv" | "try_send" | "try_recv"
            ) || expr_affects_channels(&call.receiver, fact)
                || call.args.iter().any(|a| expr_affects_channels(a, fact))
        }
        Expr::Assign(a) => {
            path_ident(&a.left)
                .map(|n| fact.is_traced(&n))
                .unwrap_or(false)
                || expr_affects_channels(&a.right, fact)
        }
        Expr::If(i) => {
            expr_affects_channels(&i.cond, fact)
                || block_affects_channels(&i.then_branch, fact)
                || i.else_branch
                    .as_ref()
                    .map(|(_, e)| expr_affects_channels(e, fact))
                    .unwrap_or(false)
        }
        Expr::Match(m) => {
            expr_affects_channels(&m.expr, fact)
                || m.arms
                    .iter()
                    .any(|a| expr_affects_channels(&a.body, fact))
        }
        Expr::While(w) => {
            expr_affects_channels(&w.cond, fact) || block_affects_channels(&w.body, fact)
        }
        Expr::ForLoop(f) => {
            expr_affects_channels(&f.expr, fact) || block_affects_channels(&f.body, fact)
        }
        Expr::Loop(l) => block_affects_channels(&l.body, fact),
        Expr::Block(b) => block_affects_channels(&b.block, fact),
        Expr::Reference(r) => expr_affects_channels(&r.expr, fact),
        Expr::Paren(p) => expr_affects_channels(&p.expr, fact),
        Expr::Field(f) => expr_affects_channels(&f.base, fact),
        Expr::Tuple(t) => t.elems.iter().any(|e| expr_affects_channels(e, fact)),
        Expr::Closure(c) => expr_affects_channels(&c.body, fact),
        Expr::Try(t) => expr_affects_channels(&t.expr, fact),
        _ => false,
    }
}

fn block_affects_channels(block: &Block, fact: &FnFact) -> bool {
    block
        .stmts
        .iter()
        .any(|stmt| match stmt {
            Stmt::Expr(e, _) => expr_affects_channels(e, fact),
            Stmt::Local(local) => local_affects_channels(local, fact),
            Stmt::Item(_) | Stmt::Macro(_) => false,
        })
}

fn local_affects_channels(local: &Local, fact: &FnFact) -> bool {
    let Some(init) = &local.init else {
        return false;
    };
    if expr_affects_channels(&init.expr, fact) {
        return true;
    }
    if let Some((a, b)) = tuple_names(&local.pat) {
        if fact.is_traced(&a) || fact.is_traced(&b) {
            return true;
        }
    }
    if let Some(name) = ident_pat(&local.pat) {
        if fact.is_traced(&name) {
            return true;
        }
    }
    false
}

fn arg_link(expr: &Expr, fact: &FnFact) -> Link {
    if let Some(name) = path_ident(expr) {
        return fact.lookup(&name).unwrap_or(Link::Unknown);
    }
    Link::Unknown
}

fn push_unique(out: &mut Vec<UncoveredSync>, item: UncoveredSync) {
    if out.iter().any(|u| u.form == item.form && u.site == item.site && u.function == item.function) {
        return;
    }
    out.push(item);
}

fn find_barrier_new(expr: &Expr) -> Option<&Expr> {
    if is_barrier_expr(expr) {
        return Some(expr);
    }
    match expr {
        Expr::Call(call) => call.args.iter().find_map(find_barrier_new),
        Expr::MethodCall(call) => find_barrier_new(&call.receiver)
            .or_else(|| call.args.iter().find_map(find_barrier_new)),
        Expr::Reference(r) => find_barrier_new(&r.expr),
        Expr::Paren(p) => find_barrier_new(&p.expr),
        _ => None,
    }
}

fn is_barrier_expr(expr: &Expr) -> bool {
    let Expr::Call(call) = expr else { return false };
    path_contains(&call.func, "Barrier") && path_last(&call.func).as_deref() == Some("new")
}

fn arc_clone_target(expr: &Expr) -> Option<String> {
    let Expr::Call(call) = expr else { return None };
    if path_last(&call.func).as_deref() != Some("clone") || !path_contains(&call.func, "Arc") {
        return None;
    }
    let arg = call.args.first()?;
    let Expr::Reference(r) = arg else { return None };
    path_ident(&r.expr)
}

fn int_literal(expr: &Expr) -> Option<i64> {
    let expr = match expr {
        Expr::Paren(p) => &p.expr,
        other => other,
    };
    let Expr::Lit(lit) = expr else { return None };
    let syn::Lit::Int(n) = &lit.lit else { return None };
    n.base10_parse::<i64>().ok()
}

fn tuple_names(pat: &Pat) -> Option<(String, String)> {
    let pat = peel_pat(pat);
    let Pat::Tuple(tuple) = pat else { return None };
    if tuple.elems.len() != 2 {
        return None;
    }
    Some((ident_pat(&tuple.elems[0])?, ident_pat(&tuple.elems[1])?))
}

fn peel_pat(pat: &Pat) -> &Pat {
    match pat {
        Pat::Type(t) => peel_pat(&t.pat),
        Pat::Reference(r) => peel_pat(&r.pat),
        other => other,
    }
}

fn ident_pat(pat: &Pat) -> Option<String> {
    match peel_pat(pat) {
        Pat::Ident(id) => Some(id.ident.to_string()),
        _ => None,
    }
}

fn path_ident(expr: &Expr) -> Option<String> {
    let Expr::Path(p) = expr else { return None };
    if p.path.segments.len() != 1 {
        return None;
    }
    Some(p.path.segments.last()?.ident.to_string())
}

fn path_last(expr: &Expr) -> Option<String> {
    let Expr::Path(p) = expr else { return None };
    Some(p.path.segments.last()?.ident.to_string())
}

fn path_contains(expr: &Expr, needle: &str) -> bool {
    let Expr::Path(p) = expr else { return false };
    p.path.segments.iter().any(|seg| seg.ident == needle)
}

fn line_starts(src: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn offset(starts: &[usize], lc: proc_macro2::LineColumn) -> String {
    let line = lc.line.saturating_sub(1).min(starts.len().saturating_sub(1));
    (starts[line] + lc.column).to_string()
}

#[cfg(test)]
mod endpoint_identity_tests {
    use super::*;

    fn endpoints(src: &str) -> Vec<(String, String)> {
        let facts = analyze(src).expect("parse");
        facts
            .endpoints
            .iter()
            .map(|e| (e.name.clone(), e.status.clone()))
            .collect()
    }

    #[test]
    fn unrelated_sequential_if_does_not_erase_endpoints() {
        let src = r#"
use std::sync::mpsc;
use std::thread;
fn main() {
    let (tx, rx) = mpsc::sync_channel::<i32>(0);
    let s = thread::spawn(move || { tx.send(1).unwrap(); });
    let r = thread::spawn(move || { rx.recv().unwrap(); });
    let x = 5;
    if x > 3 { let _ = x + 1; }
    s.join().unwrap();
    r.join().unwrap();
}
"#;
        let e = endpoints(src);
        assert!(e.iter().any(|(n, s)| n == "tx" && s == "traced"), "{e:?}");
        assert!(e.iter().any(|(n, s)| n == "rx" && s == "traced"), "{e:?}");
    }

    #[test]
    fn discarding_an_endpoint_in_a_closure_does_not_erase_others() {
        let src = r#"
use std::sync::mpsc;
use std::thread;
fn main() {
    let (tx1, rx1) = mpsc::sync_channel::<i32>(0);
    let (tx2, _rx2) = mpsc::sync_channel::<i32>(0);
    let s = thread::spawn(move || { tx1.send(1).unwrap(); let _ = tx2; });
    let r = thread::spawn(move || { let _ = rx1; });
    s.join().unwrap();
    r.join().unwrap();
}
"#;
        let e = endpoints(src);
        assert!(e.iter().any(|(n, s)| n == "tx1" && s == "traced"), "{e:?}");
    }

    #[test]
    fn conditional_channel_op_makes_other_endpoints_unknown() {
        let src = r#"
use std::sync::mpsc;
fn main() {
    let (tx, rx) = mpsc::sync_channel::<i32>(0);
    let (tx2, _rx2) = mpsc::sync_channel::<i32>(0);
    let x = 5;
    if x > 3 { tx2.send(1).unwrap(); }
    rx.recv().unwrap();
}
"#;
        let e = endpoints(src);
        assert!(e.iter().any(|(n, s)| n == "rx" && s == "unknown"), "{e:?}");
    }

    #[test]
    fn rebinding_an_endpoint_in_a_branch_makes_it_unknown() {
        let src = r#"
use std::sync::mpsc;
fn main() {
    let (tx, rx) = mpsc::sync_channel::<i32>(0);
    let (tx2, _rx2) = mpsc::sync_channel::<i32>(1);
    let x = 5;
    let mut t = tx;
    if x > 3 { t = tx2; }
    t.send(1).unwrap();
    let _ = rx;
}
"#;
        let e = endpoints(src);
        assert!(e.iter().any(|(n, s)| n == "t" && s == "unknown"), "{e:?}");
    }
}
