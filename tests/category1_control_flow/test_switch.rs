use crate::common;
use cvn::model::{TransitionId, TransitionKind};

#[test]
fn switch_creates_per_label_transitions() {
    let net = common::translate_fixture("switch.json");

    let t_init = net.transition(&TransitionId::new("main_s5_switch_Init")).unwrap();
    let t_run = net.transition(&TransitionId::new("main_s5_switch_Running")).unwrap();
    let t_done = net.transition(&TransitionId::new("main_s5_switch_Done")).unwrap();

    assert!(matches!(t_init.kind, TransitionKind::Switch { ref label } if label == "Init"));
    assert!(matches!(t_run.kind, TransitionKind::Switch { ref label } if label == "Running"));
    assert!(matches!(t_done.kind, TransitionKind::Switch { ref label } if label == "Done"));
}

#[test]
fn switch_all_share_input() {
    let net = common::translate_fixture("switch.json");

    for label in &["Init", "Running", "Done"] {
        let tid = TransitionId::new(format!("main_s5_switch_{label}"));
        let arcs = net.input_arcs(&tid);
        assert!(!arcs.is_empty());
        assert_eq!(arcs[0].place.0, "cp_main_s5");
    }
}

#[test]
fn switch_targets_correct_places() {
    let net = common::translate_fixture("switch.json");

    let pairs = [("Init", "cp_main_s6"), ("Running", "cp_main_s7"), ("Done", "cp_main_s8")];
    for (label, expected_cp) in &pairs {
        let tid = TransitionId::new(format!("main_s5_switch_{label}"));
        let out = net.output_arcs(&tid);
        assert_eq!(out[0].place.0, *expected_cp);
    }
}
