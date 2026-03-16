use crate::common;
use cvn::model::Val;

#[test]
fn var_not_in_places() {
    let net = common::translate_fixture("var_atomic.json");
    assert!(!common::has_place(&net, "rp_count"));
    assert!(!common::has_place(&net, "rp_flag"));
}

#[test]
fn var_in_initial_vars() {
    let net = common::translate_fixture("var_atomic.json");
    let vars = net.initial_vars();
    assert_eq!(vars.get("count"), Some(&Val::int(0)));
    assert_eq!(vars.get("flag"), Some(&Val::bool(false)));
}

#[test]
fn var_write_produces_update() {
    let net = common::translate_fixture("var_atomic.json");

    let tid = cvn::model::TransitionId::new("main_s1_var_write");
    let out = net.output_arcs(&tid);
    assert!(!out.is_empty());
    let update = out[0].update.as_ref().unwrap();
    assert!(update.contains_key("count"));
}
