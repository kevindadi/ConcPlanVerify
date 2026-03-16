use crate::common;

#[test]
fn semaphore_initial_tokens() {
    let net = common::translate_fixture("semaphore.json");
    assert_eq!(common::initial_tokens(&net, "rp_sem"), 2);
}

#[test]
fn semaphore_acquire_release() {
    let net = common::translate_fixture("semaphore.json");

    // Acquire = lock, Release = unlock.
    let tid_acq = cvn::model::TransitionId::new("main_s1_lock");
    let tid_rel = cvn::model::TransitionId::new("main_s2_unlock");
    assert!(net.transition(&tid_acq).is_some());
    assert!(net.transition(&tid_rel).is_some());
}
