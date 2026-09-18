use concir::ast::Program;
use concir::explore::contract::ContractSpec;
use concir::explore::{verify_program, EngineKind};
use concir::repair::search::{run_search, RepairStrategy, SearchConfig, program_fingerprint};
use concir::repair::patch::{function_hash, CirPatch, PatchChange};
use serde_json::json;
fn main() {
 let root="/Users/kevin/local-repos/ConcIR/tests/repro_bench";
 let p:Program=serde_json::from_str(&std::fs::read_to_string(format!("{root}/two_cycles.json")).unwrap()).unwrap();
 let spec:ContractSpec=serde_json::from_str(&std::fs::read_to_string(format!("{root}/two_cycles_contract.json")).unwrap()).unwrap();
 let mut rows=Vec::new();
 for case in ["default","depth_one","edits_one","verification_zero","bounds_one","verification_seven"] {
  let mut cfg=SearchConfig{strategy:RepairStrategy::Composite,..SearchConfig::default()};
  match case {"depth_one"=>cfg.max_depth=1,"edits_one"=>cfg.max_total_edits=1,"verification_zero"=>cfg.verification_budget=0,"bounds_one"=>{cfg.bounds.max_states=1;cfg.bounds.max_depth=1;},"verification_seven"=>cfg.verification_budget=7,_=>{}}
  let report=run_search(&p,&spec,&cfg);
  rows.push(json!({"case":case,"outcome":report.outcome,"stop_reason":report.stop_reason,"verifications":report.verifications,"states":report.states_explored,"patches":report.patch_chain.len(),"nodes":report.nodes}));
 }
 let mut bounded_spec=spec.clone();bounded_spec.bounds.max_states=1;bounded_spec.bounds.max_depth=1;
 let report=verify_program(&p,&bounded_spec,EngineKind::Petri);
 rows.push(json!({"case":"contract_bounds_one","outcome":report.outcome,"states":report.states_explored}));
 let single:Program=serde_json::from_str(&std::fs::read_to_string(format!("{root}/single_cycle.json")).unwrap()).unwrap();
 let patch=CirPatch{id:"audit-swap".into(),module:"main".into(),function:"t1".into(),original_hash:function_hash(&single,"main","t1").unwrap(),changes:vec![PatchChange::SwapStatements{a:"s1".into(),b:"s2".into()}],provenance:vec![]};
 std::fs::write("/private/tmp/concir-audit-repair-loop/patches.json",serde_json::to_string_pretty(&vec![patch]).unwrap()).unwrap();

 let unfix:Program=serde_json::from_str(&std::fs::read_to_string(format!("{root}/preserved_unfixable.json")).unwrap()).unwrap();
 let mut configurations=Vec::new();
 for mask in 0..4 {
  let mut candidate=unfix.clone();
  for bit in 0..2 { if mask & (1 << bit) != 0 { candidate.modules[0].functions[bit+1].body.swap(0,1); } }
  configurations.push(json!({"swapped_functions_mask":mask,"fingerprint":program_fingerprint(&candidate)}));
 }
 rows.push(json!({"case":"parent_identity_map","configurations":configurations}));
 println!("{}",serde_json::to_string_pretty(&rows).unwrap());
}
