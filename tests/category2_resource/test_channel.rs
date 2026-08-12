use crate::common;
use unipn::model::TransitionKind;

#[test]
fn channel_initial_empty() {
    let net = common::translate_fixture("channel.json");
    assert!(common::has_place(&net, "ch"));
    assert_eq!(common::initial_tokens(&net, "ch"), 0);
}

#[test]
fn channel_send_produces_token() {
    let net = common::translate_fixture("channel.json");

    assert!(
        common::transition_kind(&net, "sender_s1_send").is_some_and(|k| k == TransitionKind::Send)
    );

    let out = common::output_arcs(&net, "sender_s1_send");
    assert!(out.iter().any(|(n, _)| n == "ch"));
}

#[test]
fn channel_recv_consumes_token() {
    let net = common::translate_fixture("channel.json");

    assert!(
        common::transition_kind(&net, "receiver_s1_recv")
            .is_some_and(|k| k == TransitionKind::Recv)
    );

    let in_arcs = common::input_arcs(&net, "receiver_s1_recv");
    assert!(in_arcs.iter().any(|(n, _)| n == "ch"));
}
