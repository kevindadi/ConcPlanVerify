use std::collections::{HashMap, HashSet};

use cvn::builder::CvnNetBuilder;
use cvn::model::{BoolExpr, ResourceType, TransitionKind, Val, VarUpdate};

use crate::error::TranslateError;

// ── Naming helpers ──────────────────────────────────────────────────────────

/// Control place id: `cp_{fn_name}_{sid}`
pub(crate) fn cp_id(fn_name: &str, sid: &str) -> String {
    format!("cp_{fn_name}_{sid}")
}

/// Resource place id: `rp_{res_name}`
pub(crate) fn rp_id(res_name: &str) -> String {
    format!("rp_{res_name}")
}

/// Wait place id: `wp_{cv_name}_{fn_name}_{sid}`
pub(crate) fn wp_id(cv_name: &str, fn_name: &str, sid: &str) -> String {
    format!("wp_{cv_name}_{fn_name}_{sid}")
}

/// Reacquire place id: `ra_{fn_name}_{sid}`
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

/// Classification of a CIR resource for translation purposes.
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
    builder: CvnNetBuilder,

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

    /// Function currently being translated. Attached to every transition as
    /// `source_function` so repair can attribute behavior (including synthetic
    /// transitions) to a CIR function without re-scanning the program.
    current_function: Option<String>,

    /// Errors collected during translation.
    pub(crate) errors: Vec<TranslateError>,
}

impl TranslateContext {
    pub(crate) fn new() -> Self {
        Self {
            builder: CvnNetBuilder::new(),
            control_places: HashSet::new(),
            resource_map: HashMap::new(),
            all_enum_variants: HashSet::new(),
            rwlock_n: 1,
            lock_tracker: HashMap::new(),
            wait_sites: HashMap::new(),
            post_wait_locks: HashMap::new(),
            current_function: None,
            errors: Vec::new(),
        }
    }

    // ── Builder delegation (uses std::mem::take for consuming-self API) ──

    fn take_builder(&mut self) -> CvnNetBuilder {
        std::mem::take(&mut self.builder)
    }

    pub(crate) fn add_control_place(&mut self, fn_name: &str, sid: &str) {
        let id = cp_id(fn_name, sid);
        self.builder = self.take_builder().add_control_place(&id, fn_name, sid);
    }

    pub(crate) fn add_resource_place(&mut self, res_name: &str, resource_type: ResourceType) {
        let id = rp_id(res_name);
        self.builder = self
            .take_builder()
            .add_resource_place(&id, res_name, resource_type);
    }

    pub(crate) fn add_wait_place(&mut self, cv_name: &str, fn_name: &str, sid: &str) {
        let id = wp_id(cv_name, fn_name, sid);
        self.builder = self
            .take_builder()
            .add_wait_place(&id, cv_name, fn_name, sid);
    }

    pub(crate) fn set_return(&mut self, place_id: &str) {
        self.builder = self.take_builder().set_return(place_id);
    }

    pub(crate) fn add_transition(&mut self, id: &str, kind: TransitionKind, sids: &[&str]) {
        match self.current_function.clone() {
            Some(fn_name) => {
                self.builder = self
                    .take_builder()
                    .add_transition_with_source(id, kind, sids, fn_name)
            }
            None => {
                self.builder = self
                    .take_builder()
                    .add_transition_with_anchor(id, kind, sids)
            }
        };
    }

    /// Set the function whose statements are being translated. All
    /// transitions added until the next call are attributed to it.
    pub(crate) fn set_current_function(&mut self, fn_name: &str) {
        self.current_function = Some(fn_name.to_string());
    }

    /// Assign a disjunctive family to a previously added transition.
    pub(crate) fn set_disjunctive_family(&mut self, transition_id: &str, family: &str) {
        self.builder = self
            .take_builder()
            .set_disjunctive_family(transition_id, family);
    }

    pub(crate) fn add_input_arc(
        &mut self,
        place_id: &str,
        transition_id: &str,
        weight: u32,
        guard: BoolExpr,
    ) {
        self.builder = self
            .take_builder()
            .add_input_arc(place_id, transition_id, weight, guard);
    }

    pub(crate) fn add_output_arc(
        &mut self,
        transition_id: &str,
        place_id: &str,
        weight: u32,
        update: Option<VarUpdate>,
    ) {
        self.builder = self
            .take_builder()
            .add_output_arc(transition_id, place_id, weight, update);
    }

    pub(crate) fn set_initial_tokens(&mut self, place_id: &str, count: u32) {
        self.builder = self.take_builder().set_initial_tokens(place_id, count);
    }

    pub(crate) fn add_variable(&mut self, name: &str, initial_value: Val) {
        self.builder = self.take_builder().add_variable(name, initial_value);
    }

    // ── Control place management ────────────────────────────────────────

    /// Ensure a control place exists for `(fn_name, sid)`. No-op if already added.
    pub(crate) fn ensure_control_place(&mut self, fn_name: &str, sid: &str) {
        let key = (fn_name.to_string(), sid.to_string());
        if self.control_places.insert(key) {
            self.add_control_place(fn_name, sid);
        }
    }

    /// Ensure a reacquire place `ra_{fn_name}_{sid}` exists. Modeled as a
    /// control place with a distinctive id prefix.
    pub(crate) fn ensure_reacquire_place(&mut self, fn_name: &str, sid: &str) {
        let ra_sid = format!("{sid}_ra");
        let key = (fn_name.to_string(), ra_sid.clone());
        if self.control_places.insert(key) {
            let id = ra_id(fn_name, sid);
            self.builder = self.take_builder().add_control_place(&id, fn_name, &ra_sid);
        }
    }

    /// Ensure the return place exists for a function.
    pub(crate) fn ensure_return_place(&mut self, fn_name: &str) {
        let key = (fn_name.to_string(), "ret".to_string());
        if self.control_places.insert(key) {
            self.add_control_place(fn_name, "ret");
            let id = cp_id(fn_name, "ret");
            self.set_return(&id);
        }
    }

    // ── Error collection ────────────────────────────────────────────────

    pub(crate) fn push_error(&mut self, err: TranslateError) {
        self.errors.push(err);
    }

    pub(crate) fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Consume the context and return either the built CvnNet or accumulated errors.
    pub(crate) fn finish(self) -> Result<cvn::net::CvnNet, Vec<TranslateError>> {
        if self.has_errors() {
            return Err(self.errors);
        }
        self.builder.build_with_anchor_check().map_err(|cvn_errs| {
            cvn_errs
                .into_iter()
                .map(|e| TranslateError::BuilderError(e.to_string()))
                .collect()
        })
    }
}
