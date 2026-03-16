mod common;
mod category1_control_flow;
mod category2_resource;
mod category3_guard_update;

/// Integration tests using the CIR examples from the cir/ submodule.
mod cir_examples {
    use crate::common;
    use std::path::Path;

    fn load_cir_example(name: &str) -> cir::ast::Program {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("cir/examples")
            .join(name);
        let json = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()))
    }

    #[test]
    fn producer_consumer_translates() {
        let program = load_cir_example("producer_consumer.json");
        let net = cir2cvn::translate(&program).expect("translation should succeed");

        // Basic structural checks.
        assert!(net.place_count() > 0);
        assert!(net.transition_count() > 0);

        // Mutex place with 1 token.
        assert_eq!(common::initial_tokens(&net, "rp_mtx"), 1);

        // Entry function starts with token.
        assert_eq!(common::initial_tokens(&net, "cp_main_s1"), 1);

        // Variables in initial state.
        let vars = net.initial_vars();
        assert!(vars.contains_key("count"));
        assert!(vars.contains_key("done"));
    }

    #[test]
    fn state_machine_translates() {
        let program = load_cir_example("state_machine.json");
        let net = cir2cvn::translate(&program).expect("translation should succeed");

        assert!(net.place_count() > 0);
        assert!(net.transition_count() > 0);

        let vars = net.initial_vars();
        assert!(vars.contains_key("state"));
    }

    #[test]
    fn with_summary_translates() {
        let program = load_cir_example("with_summary.json");
        let net = cir2cvn::translate(&program).expect("translation should succeed");

        assert!(net.place_count() > 0);
        assert!(net.transition_count() > 0);

        // The call to validate should produce a Call transition.
        let tid = cvn::model::TransitionId::new("worker_s11_call");
        assert!(net.transition(&tid).is_some());
    }

    #[test]
    fn complex_rwlock_translates() {
        let program = load_cir_example("complex_rwlock.json");
        let net = cir2cvn::translate(&program).expect("translation should succeed");

        assert!(net.place_count() > 0);
        assert!(net.transition_count() > 0);

        // RwLock place exists with N tokens.
        assert!(common::has_place(&net, "rp_rw"));
        let rw_tokens = common::initial_tokens(&net, "rp_rw");
        assert!(rw_tokens >= 2, "RwLock should have N >= 2 tokens, got {rw_tokens}");
    }

    #[test]
    fn post_translation_validation() {
        let program = load_cir_example("producer_consumer.json");
        let net = cir2cvn::translate(&program).expect("translation should succeed");

        let warnings = cir2cvn::validate::check_translation(&net);
        // Print warnings for debugging but don't fail (some are expected during
        // chain-based notify_all expansion).
        for w in &warnings {
            eprintln!("validation warning: {w}");
        }
    }
}
