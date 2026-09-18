use std::collections::{VecDeque,HashSet};
use concir::sem::system::TransitionSystem;
use concir::sem::outcome::AnalysisBounds;
use concir::petri::net::{PlaceKey,NetToken};
fn main() {
    let path=std::env::args().nth(1).expect("model path");
    let p:concir::ast::Program=serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    println!("check_valid={}",concir::validate::validate(&p).valid);
    let sp=concir::sem::program::lower(&p).unwrap();
    let i=concir::interp::Interpreter::new(&sp,AnalysisBounds::default());
    let mut q=VecDeque::from([i.initial().unwrap()]); let mut seen=HashSet::new();
    while let Some(s)=q.pop_front(){
        if !seen.insert(s.clone()){continue;}
        let enabled=i.successors(&s).unwrap();
        if s.store.channels.values().any(|c| c.pending_recv.len()==2){
            println!("interp: two waiting receivers, {} successors",enabled.steps.len());
            for step in enabled.steps {println!("interp: remaining_waiter={:?}",step.state.store.channels.values().next().unwrap().pending_recv);}
            break;
        }
        for st in enabled.steps {q.push_back(st.state);}
    }
    let n=concir::petri::PetriEngine::new(&sp,AnalysisBounds::default());
    let mut q=VecDeque::from([n.initial().unwrap()]);let mut seen=HashSet::new();
    while let Some(s)=q.pop_front(){
        if !seen.insert(s.clone()){continue;}
        let enabled=n.successors(&s).unwrap();
        let pending=n.net.places.iter().any(|p| matches!(p.key,PlaceKey::ChannelRecv(_)) && s.place_tokens(p.id).len()==2);
        if pending {
            println!("petri: two waiting receivers, {} successors",enabled.steps.len());
            for step in enabled.steps { let waiting:Vec<_>=step.state.marking.values().flatten().filter(|t|matches!(t,NetToken::RecvWait{..})).collect();println!("petri: remaining_waiter={waiting:?}");}
            break;
        }
        for st in enabled.steps {q.push_back(st.state);}
    }
}
