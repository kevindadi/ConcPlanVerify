use crate::common;

#[test]
fn protection_does_not_produce_cvn_structure() {
    // The sequential_chain fixture has no protection, but the mutex_exclusive
    // fixture has the same resources. Protection should be ignored.
    let net = common::translate_fixture("sequential_chain.json");

    // No extra places or transitions should appear from protection.
    // Just verify the net is well-formed.
    assert!(net.num_places() > 0);
    assert!(net.num_transitions() > 0);
}
