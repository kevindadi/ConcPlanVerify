use crate::common;
use unipn::Val;

#[test]
fn var_not_in_places() {
    let net = common::translate_fixture("var_atomic.json");
    assert!(!common::has_place(&net, "count"));
    assert!(!common::has_place(&net, "flag"));
}

#[test]
fn var_in_initial_vars() {
    let net = common::translate_fixture("var_atomic.json");
    let vars = common::initial_vars(&net);
    assert_eq!(vars.get("count"), Some(&Val::int(0)));
    assert_eq!(vars.get("flag"), Some(&Val::bool(false)));
}

#[test]
fn var_write_produces_update() {
    let net = common::translate_fixture("var_atomic.json");

    let out = common::output_arcs(&net, "main_s1_var_write");
    assert!(!out.is_empty());
    let update = common::output_update_by_name(&net, "main_s1_var_write", &out[0].0)
        .expect("should have update");
    assert!(update.contains_key("count"));
}
