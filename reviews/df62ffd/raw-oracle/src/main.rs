use std::collections::{HashSet, VecDeque, HashMap};
use concir::sem::system::{TransitionSystem,Predicate};
use concir::sem::outcome::AnalysisBounds;
use concir::explore::contract::{ContractSpec,Property};
fn exact<S:TransitionSystem>(engine:&S, goal:&Predicate, name:&str) {
    let mut seen=HashSet::new(); let mut q=VecDeque::from([engine.initial().unwrap()]);
    let mut keys=HashMap::new(); let mut reachable=false;let mut conflicting=0;
    while let Some(state)=q.pop_front(){
        if !seen.insert(state.clone()){continue;}
        assert!(seen.len()<10000,"oracle fixture must be finite");
        let sat=engine.satisfied(&state,goal);reachable|=sat;
        let key=engine.state_key(&state);
        if let Some(old)=keys.insert(key,sat){if old!=sat{conflicting+=1;}}
        let next=engine.successors(&state).unwrap();
        assert!(next.boundary.is_empty());
        for s in next.steps{q.push_back(s.state);}
    }
    println!("{name}: raw_states={} goal_reachable={reachable} equal_key_with_different_predicate_truth={conflicting}",seen.len());
}
fn main(){
    let dir=std::env::args().nth(1).unwrap();
    let program:concir::ast::Program=serde_json::from_str(&std::fs::read_to_string(format!("{dir}/r3_canonical_collision_A.json")).unwrap()).unwrap();
    let sp=concir::sem::program::lower(&program).unwrap();
    for name in ["A","B"]{
        let spec:ContractSpec=serde_json::from_str(&std::fs::read_to_string(format!("{dir}/r3_canonical_collision_{name}_contract.json")).unwrap()).unwrap();
        let contract=spec.resolve(&sp).unwrap();
        let Property::Reachability{goal}=&contract.properties[0].property else{panic!()};
        exact(&concir::interp::Interpreter::new(&sp,AnalysisBounds::default()),goal,&format!("interp-{name}"));
        exact(&concir::petri::PetriEngine::new(&sp,AnalysisBounds::default()),goal,&format!("petri-{name}"));
    }
}
