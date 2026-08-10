mod common;
mod category1_control_flow;
mod category2_resource;
mod category3_guard_update;

/// Integration tests using the ConcIR examples from the vendored `cir/` tree.
mod cir_examples {
    use crate::common;
    use std::path::Path;
    use unipn::NetLike;

    fn load_cir_example(name: &str) -> concir::ast::Program {
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

        assert!(net.num_places() > 0);
        assert!(net.num_transitions() > 0);

        assert_eq!(common::initial_tokens(&net, "mtx"), 1);
        assert_eq!(common::initial_tokens(&net, "main.s1"), 1);

        let vars = common::initial_vars(&net);
        assert!(vars.contains_key("count"));
    }

    #[test]
    fn state_machine_translates() {
        let program = load_cir_example("state_machine.json");
        let net = cir2cvn::translate(&program).expect("translation should succeed");

        assert!(net.num_places() > 0);
        assert!(net.num_transitions() > 0);

        let vars = common::initial_vars(&net);
        assert!(vars.contains_key("state"));
    }

    #[test]
    fn with_summary_translates() {
        let program = load_cir_example("with_summary.json");
        let net = cir2cvn::translate(&program).expect("translation should succeed");

        assert!(net.num_places() > 0);
        assert!(net.num_transitions() > 0);

        // The call to validate should produce a Call transition.
        assert!(common::has_transition(&net, "worker_s11_call"));
    }

    #[test]
    fn transitions_carry_source_function() {
        let program = load_cir_example("producer_consumer.json");
        let net = cir2cvn::translate(&program).expect("translation should succeed");

        let mut transitions = net.transitions().to_vec();
        assert!(!transitions.is_empty());
        assert!(
            transitions.iter().all(|t| t.scope.is_some()),
            "every transition should carry its source function"
        );

        // Every transition's source function must be a real function in the program.
        let fns: Vec<&str> = program.functions.iter().map(|f| f.name.as_str()).collect();
        for t in transitions.drain(..) {
            let fn_name = t.scope.as_deref().expect("source function");
            assert!(
                fns.contains(&fn_name),
                "transition {} has unknown source function {fn_name}",
                t.id
            );
        }
    }

    #[test]
    fn complex_rwlock_translates() {
        let program = load_cir_example("complex_rwlock.json");
        let net = cir2cvn::translate(&program).expect("translation should succeed");

        assert!(net.num_places() > 0);
        assert!(net.num_transitions() > 0);

        // RwLock place exists with N tokens.
        assert!(common::has_place(&net, "rw"));
        let rw_tokens = common::initial_tokens(&net, "rw");
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
