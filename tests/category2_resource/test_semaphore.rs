use crate::common;
use cvn::model::{TransitionId, TransitionKind};

#[test]
fn semaphore_initial_tokens() {
    let net = common::translate_fixture("semaphore.json");
    assert_eq!(common::initial_tokens(&net, "rp_sem"), 2);
}

#[test]
fn semaphore_acquire_release() {
    let net = common::translate_fixture("semaphore.json");

    let tid_acq = TransitionId::new("main_s1_acquire");
    let tid_rel = TransitionId::new("main_s2_release");
    let t_acq = net.transition(&tid_acq).expect("acquire transition");
    let t_rel = net.transition(&tid_rel).expect("release transition");
    assert!(matches!(t_acq.kind, TransitionKind::Acquire));
    assert!(matches!(t_rel.kind, TransitionKind::Release));
}
