use crate::common;
use unipn::model::TransitionKind;

#[test]
fn semaphore_initial_tokens() {
    let net = common::translate_fixture("semaphore.json");
    assert_eq!(common::initial_tokens(&net, "sem"), 2);
}

#[test]
fn semaphore_acquire_release() {
    let net = common::translate_fixture("semaphore.json");

    let k_acq = common::transition_kind(&net, "main_s1_acquire").unwrap();
    let k_rel = common::transition_kind(&net, "main_s2_release").unwrap();
    assert_eq!(k_acq, TransitionKind::Acquire);
    assert_eq!(k_rel, TransitionKind::Release);
}
