use crate::common;
use cvn::model::{PlaceId, TransitionId, TransitionKind};

#[test]
fn channel_initial_empty() {
    let net = common::translate_fixture("channel.json");
    assert!(common::has_place(&net, "rp_ch"));
    assert_eq!(common::initial_tokens(&net, "rp_ch"), 0);
}

#[test]
fn channel_send_produces_token() {
    let net = common::translate_fixture("channel.json");

    let tid = TransitionId::new("sender_s1_send");
    let t = net.transition(&tid).unwrap();
    assert!(matches!(t.kind, TransitionKind::Send));

    let out = net.output_arcs(&tid);
    assert!(out.iter().any(|a| a.place == PlaceId::new("rp_ch")));
}

#[test]
fn channel_recv_consumes_token() {
    let net = common::translate_fixture("channel.json");

    let tid = TransitionId::new("receiver_s1_recv");
    let t = net.transition(&tid).unwrap();
    assert!(matches!(t.kind, TransitionKind::Recv));

    let in_arcs = net.input_arcs(&tid);
    assert!(in_arcs.iter().any(|a| a.place == PlaceId::new("rp_ch")));
}
