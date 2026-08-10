use crate::common;
use unipn::model::TransitionKind;

#[test]
fn switch_creates_per_label_transitions() {
    let net = common::translate_fixture("switch.json");

    let k_init = common::transition_kind(&net, "main_s5_switch_Init").unwrap();
    let k_run = common::transition_kind(&net, "main_s5_switch_Running").unwrap();
    let k_done = common::transition_kind(&net, "main_s5_switch_Done").unwrap();

    assert!(matches!(k_init, TransitionKind::Switch { ref label } if label == "Init"));
    assert!(matches!(k_run, TransitionKind::Switch { ref label } if label == "Running"));
    assert!(matches!(k_done, TransitionKind::Switch { ref label } if label == "Done"));
}

#[test]
fn switch_all_share_input() {
    let net = common::translate_fixture("switch.json");

    for label in &["Init", "Running", "Done"] {
        let name = format!("main_s5_switch_{label}");
        let arcs = common::input_arcs(&net, &name);
        assert!(!arcs.is_empty());
        assert_eq!(arcs[0].0, "main.s5");
    }
}

#[test]
fn switch_targets_correct_places() {
    let net = common::translate_fixture("switch.json");

    let pairs = [("Init", "main.s6"), ("Running", "main.s7"), ("Done", "main.s8")];
    for (label, expected_cp) in &pairs {
        let name = format!("main_s5_switch_{label}");
        let out = common::output_arcs(&net, &name);
        assert_eq!(out[0].0, *expected_cp);
    }
}
