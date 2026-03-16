use std::path::Path;

use cir::ast::Program;
use cvn::model::PlaceId;
use cvn::net::CvnNet;

pub fn load_fixture(name: &str) -> Program {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to parse fixture {}: {e}", path.display()))
}

pub fn translate_fixture(name: &str) -> CvnNet {
    let program = load_fixture(name);
    cir2cvn::translate(&program).unwrap_or_else(|errs| {
        panic!(
            "translation failed for {name}: {:?}",
            errs.iter().map(|e| e.to_string()).collect::<Vec<_>>()
        )
    })
}

pub fn has_place(net: &CvnNet, id: &str) -> bool {
    net.place(&PlaceId::new(id)).is_some()
}

pub fn initial_tokens(net: &CvnNet, place_id: &str) -> u32 {
    net.initial_marking()
        .get(&PlaceId::new(place_id))
        .copied()
        .unwrap_or(0)
}

pub fn transition_count(net: &CvnNet) -> usize {
    net.transition_count()
}

pub fn place_count(net: &CvnNet) -> usize {
    net.place_count()
}
