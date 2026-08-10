use crate::common;

#[test]
fn loop_has_back_edge() {
    let net = common::translate_fixture("loop_back_edge.json");

    // s3 → s1 is the back edge (return op with transfer next s1).
    assert!(common::has_transition(&net, "main_s3_return"));

    let out = common::output_arcs(&net, "main_s3_return");
    // Should point back to main.s1.
    assert!(out.iter().any(|(n, _)| n == "main.s1"));
}

#[test]
fn loop_branch_at_s1() {
    let net = common::translate_fixture("loop_back_edge.json");

    assert!(common::has_transition(&net, "main_s1_branch_true"));
    assert!(common::has_transition(&net, "main_s1_branch_false"));
}

#[test]
fn loop_var_write_update() {
    let net = common::translate_fixture("loop_back_edge.json");

    let out = common::output_arcs(&net, "main_s2_var_write");
    assert!(!out.is_empty());

    let update = common::output_update_by_name(&net, "main_s2_var_write", &out[0].0)
        .expect("should have update");
    assert!(update.contains_key("i"));
}
