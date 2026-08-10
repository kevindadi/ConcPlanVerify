use std::collections::{HashMap, HashSet};

use unipn::model::{ControlSub, PlaceKind, ResourceType, TransitionKind};
use unipn::{BoolExpr, Net, NetBuilder, PlaceId, TransitionId, Val, VarUpdate};

use crate::error::TranslateError;

// ── Naming helpers ──────────────────────────────────────────────────────────
//
// These produce the *map keys* used across the translator. The actual
// `Place::name` / `Transition::name` stored on the built net use parse-friendly
// forms (see `add_control_place` etc.) so the repair layer can recover
// (fn, sid) / res_name / cv_name without re-scanning the ConcIR.

/// Control place key: `cp_{fn_name}_{sid}`
pub(crate) fn cp_id(fn_name: &str, sid: &str) -> String {
    format!("cp_{fn_name}_{sid}")
}

/// Resource place key: `rp_{res_name}`
pub(crate) fn rp_id(res_name: &str) -> String {
    format!("rp_{res_name}")
}

/// Wait place key: `wp_{cv_name}_{fn_name}_{sid}`
pub(crate) fn wp_id(cv_name: &str, fn_name: &str, sid: &str) -> String {
    format!("wp_{cv_name}_{fn_name}_{sid}")
}

/// Reacquire place key: `ra_{fn_name}_{sid}`
pub(crate) fn ra_id(fn_name: &str, wait_sid: &str) -> String {
    format!("ra_{fn_name}_{wait_sid}")
}

/// Transition id: `{fn_name}_{sid}_{suffix}`
pub(crate) fn tid(fn_name: &str, sid: &str, suffix: &str) -> String {
    format!("{fn_name}_{sid}_{suffix}")
}

/// Condvar waiter-count variable name: `nw_{cv_name}`
pub(crate) fn nw_var_name(cv_name: &str) -> String {
    format!("nw_{cv_name}")
}

/// Per-wait-site notify-all flag variable name: `na_{fn_name}_{sid}`
pub(crate) fn na_var_name(fn_name: &str, sid: &str) -> String {
    format!("na_{fn_name}_{sid}")
}

// ── Resource info ───────────────────────────────────────────────────────────

/// Classification of a ConcIR resource for translation purposes.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum ResKind {
    Mutex,
    RwLock,
    Condvar,
    Semaphore { count: u32 },
    Channel,
    Var { enum_variants: Vec<String> },
    Atomic { enum_variants: Vec<String> },
}

/// Read vs. write lock tracking for RwLock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LockKind {
    Read,
    Write,
}

/// Info about a condvar wait-site, collected during phase 2 pre-scan.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct WaitSite {
    pub fn_name: String,
    pub sid: String,
    pub mutex: String,
}

// ── TranslateContext ────────────────────────────────────────────────────────

pub(crate) struct TranslateContext {
    builder: NetBuilder,

    /// String place key → net place index.
    place_map: HashMap<String, PlaceId>,
    /// String transition key → net transition index.
    trans_map: HashMap<String, TransitionId>,

    /// Already-registered control places `(fn_name, sid)`.
    pub(crate) control_places: HashSet<(String, String)>,

    /// Resource classification: resource_name → ResKind.
    pub(crate) resource_map: HashMap<String, ResKind>,

    /// All known enum variants across all Enum-typed resources.
    pub(crate) all_enum_variants: HashSet<String>,

    /// RwLock N value (max concurrent readers = spawn count + 1).
    pub(crate) rwlock_n: u32,

    /// Per-function lock-kind tracker: `(fn_name, resource_name) → LockKind`.
    /// Used to determine weight on RwLock drop.
    pub(crate) lock_tracker: HashMap<(String, String), LockKind>,

    /// Condvar wait-sites: `cv_name → Vec<WaitSite>`.
    pub(crate) wait_sites: HashMap<String, Vec<WaitSite>>,

    /// Post-wait lock markers: `(fn_name, sid) → mutex_name`.
    /// Locks at these sites are translated as Sequential (lock already held
    /// after reacquire).
    pub(crate) post_wait_locks: HashMap<(String, String), String>,

    /// Names of body-less ("nobody") functions. These are pure placeholders
    /// (codegen markers), not call-chain elements: a `call` to one is an atomic
    /// pass-through, and the placeholder does not constrain control flow.
    pub(crate) bodyless_functions: HashSet<String>,

    /// Computation hints (`effects`) keyed by function name, applied as
    /// unknown-write updates on the atomic call pass-through for body-less
    /// callees.
    pub(crate) fn_effects: HashMap<String, concir::ast::FunctionEffects>,

    /// Typed data-flow index for call sites: callee name → modeled params in
    /// declaration order, modeled return, and the CVN variable names they map
    /// to. Projection principle: only `modeled` values enter the net.
    pub(crate) fn_dataflow: HashMap<String, FnDataFlow>,

    /// Empty alias map returned by [`TranslateContext::aliases_for`] for
    /// functions without modeled params.
    empty_aliases: HashMap<String, String>,

    /// Function currently being translated. Attached to every transition as
    /// `scope` so repair can attribute behavior (including synthetic
    /// transitions) to a ConcIR function without re-scanning the program.
    current_function: Option<String>,

    /// Errors collected during translation.
    pub(crate) errors: Vec<TranslateError>,
}

/// Data-flow signature of a function relevant to translation.
#[derive(Debug, Clone, Default)]
pub(crate) struct FnDataFlow {
    /// Modeled parameters in declaration order (argument binding order).
    pub(crate) modeled_params: Vec<concir::ast::ParamDecl>,
    /// Modeled return declaration, if any.
    pub(crate) modeled_return: Option<concir::ast::ParamDecl>,
    /// Bare parameter name → CVN variable name (`p_{fn}_{param}`).
    pub(crate) param_cvn: HashMap<String, String>,
    /// CVN variable name for the modeled return (`r_{fn}_{ret}`).
    pub(crate) return_cvn: Option<String>,
}

impl TranslateContext {
    pub(crate) fn new() -> Self {
        Self {
            builder: NetBuilder::new(),
            place_map: HashMap::new(),
            trans_map: HashMap::new(),
            control_places: HashSet::new(),
            resource_map: HashMap::new(),
            all_enum_variants: HashSet::new(),
            rwlock_n: 1,
            lock_tracker: HashMap::new(),
            wait_sites: HashMap::new(),
            post_wait_locks: HashMap::new(),
            bodyless_functions: HashSet::new(),
            fn_effects: HashMap::new(),
            fn_dataflow: HashMap::new(),
            empty_aliases: HashMap::new(),
            current_function: None,
            errors: Vec::new(),
        }
    }

    fn place_index(&mut self, key: &str) -> PlaceId {
        *self.place_map.get(key).expect("unknown place key")
    }

    fn trans_index(&mut self, key: &str) -> TransitionId {
        *self.trans_map.get(key).expect("unknown transition key")
    }

    // ── Node creation ─────────────────────────────────────────────────────

    pub(crate) fn add_control_place(&mut self, fn_name: &str, sid: &str) {
        let key = cp_id(fn_name, sid);
        if self.place_map.contains_key(&key) {
            return;
        }
        let name = format!("{fn_name}.{sid}");
        let idx = self.builder.add_place(name, PlaceKind::Control(ControlSub::Statement));
        self.place_map.insert(key, idx);
    }

    pub(crate) fn add_resource_place(&mut self, res_name: &str, resource_type: ResourceType) {
        let key = rp_id(res_name);
        if self.place_map.contains_key(&key) {
            return;
        }
        let idx = self
            .builder
            .add_place(res_name.to_string(), PlaceKind::Resource(resource_type));
        self.place_map.insert(key, idx);
    }

    pub(crate) fn add_wait_place(&mut self, cv_name: &str, fn_name: &str, sid: &str) {
        let key = wp_id(cv_name, fn_name, sid);
        if self.place_map.contains_key(&key) {
            return;
        }
        let name = format!("{cv_name}@{fn_name}.{sid}");
        let idx = self.builder.add_place(name, PlaceKind::Control(ControlSub::WaitPoint));
        self.place_map.insert(key, idx);
    }

    pub(crate) fn add_transition(&mut self, id: &str, kind: TransitionKind, sids: &[&str]) {
        let idx = self.builder.add_transition(id.to_string(), kind);
        self.trans_map.insert(id.to_string(), idx);
        for sid in sids {
            self.builder.set_anchor(idx, (*sid).to_string());
        }
        if let Some(fn_name) = &self.current_function {
            self.builder.set_scope(idx, fn_name.clone());
        }
    }

    /// Set the function whose statements are being translated. All
    /// transitions added until the next call are attributed to it.
    pub(crate) fn set_current_function(&mut self, fn_name: &str) {
        self.current_function = Some(fn_name.to_string());
    }

    /// Map bare parameter names to their CVN variable names for `fn_name`
    /// (empty when the function has no modeled parameters). Passed to the
    /// expression parser so `Ref("n")` becomes `Ref("p_f_n")`.
    pub(crate) fn aliases_for(&self, fn_name: &str) -> &HashMap<String, String> {
        self.fn_dataflow
            .get(fn_name)
            .map(|d| &d.param_cvn)
            .unwrap_or(&self.empty_aliases)
    }

    /// Assign a disjunctive family to a previously added transition.
    pub(crate) fn set_disjunctive_family(&mut self, transition_id: &str, family: &str) {
        let idx = self.trans_index(transition_id);
        self.builder.set_family(idx, family.to_string());
    }

    pub(crate) fn add_input_arc(
        &mut self,
        place_id: &str,
        transition_id: &str,
        weight: u32,
        guard: BoolExpr,
    ) {
        let p = self.place_index(place_id);
        let t = self.trans_index(transition_id);
        self.builder.add_input_arc(p, t, weight, guard);
    }

    pub(crate) fn add_output_arc(
        &mut self,
        transition_id: &str,
        place_id: &str,
        weight: u32,
        update: Option<VarUpdate>,
    ) {
        let t = self.trans_index(transition_id);
        let p = self.place_index(place_id);
        self.builder.add_output_arc(t, p, weight, update);
    }

    pub(crate) fn set_initial_tokens(&mut self, place_id: &str, count: u32) {
        let p = self.place_index(place_id);
        self.builder.set_initial_tokens(p, count);
    }

    pub(crate) fn add_variable(&mut self, name: &str, initial_value: Val) {
        self.builder.add_variable(name.to_string(), initial_value);
    }

    /// Declare the Int value domain of a variable (bounded Int base type).
    pub(crate) fn set_variable_domain(&mut self, name: &str, lo: i64, hi: i64) {
        self.builder.set_variable_domain(name.to_string(), lo, hi);
    }

    // ── Control place management ────────────────────────────────────────

    /// Ensure a control place exists for `(fn_name, sid)`. No-op if already added.
    pub(crate) fn ensure_control_place(&mut self, fn_name: &str, sid: &str) {
        let key = (fn_name.to_string(), sid.to_string());
        if self.control_places.insert(key) {
            self.add_control_place(fn_name, sid);
        }
    }

    /// Ensure a reacquire place exists. Modeled as a `Reacquire` control place.
    pub(crate) fn ensure_reacquire_place(&mut self, fn_name: &str, sid: &str) {
        let ra_sid = format!("{sid}_ra");
        let key = (fn_name.to_string(), ra_sid.clone());
        if self.control_places.insert(key) {
            let key_id = ra_id(fn_name, sid);
            let name = format!("{fn_name}.{sid}#ra");
            let idx = self
                .builder
                .add_place(name, PlaceKind::Control(ControlSub::Reacquire));
            self.place_map.insert(key_id, idx);
        }
    }

    /// Ensure the return place exists for a function (thread-terminal).
    pub(crate) fn ensure_return_place(&mut self, fn_name: &str) {
        let key = (fn_name.to_string(), "ret".to_string());
        if self.control_places.insert(key) {
            let key_id = cp_id(fn_name, "ret");
            let name = format!("{fn_name}.ret");
            let idx = self
                .builder
                .add_place(name, PlaceKind::Control(ControlSub::ThreadEnd));
            self.place_map.insert(key_id, idx);
        }
    }

    // ── Error collection ────────────────────────────────────────────────

    pub(crate) fn push_error(&mut self, err: TranslateError) {
        self.errors.push(err);
    }

    pub(crate) fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Consume the context and return either the built Net or accumulated errors.
    pub(crate) fn finish(self) -> Result<Net, Vec<TranslateError>> {
        if self.has_errors() {
            return Err(self.errors);
        }
        Ok(self.builder.build())
    }
}
