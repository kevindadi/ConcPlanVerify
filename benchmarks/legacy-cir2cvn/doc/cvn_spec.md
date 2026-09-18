# CVN Specification Reference

> See [`../cvn/README.md`](../cvn/README.md) for the CVN library documentation.

The CVN (Concurrency Verification Net) is a weighted P/T Petri net with global variable guards
(not a classical colored Petri net with token colors).
Key types are in `cvn::model`:

- `CvnNet` — the complete net (petgraph-backed)
- `Place` / `PlaceId` / `PlaceKind` — places (Control, Resource, Wait)
- `Transition` / `TransitionId` / `TransitionKind` — transitions with kind labels;
  optional `disjunctive_family` groups mutually exclusive translation variants for
  dead-transition analysis (see [`condvar_modeling.md`](condvar_modeling.md))
- `InputArcData` / `OutputArcData` — arcs with weight, guard, and update
- `BoolExpr` / `Expr` — guard and value expressions
- `Val` / `ConcreteVal` — values (Concrete or Unknown)
- `State` / `Marking` / `VarStore` — runtime state for analysis
- `CvnNetBuilder` — builder API for constructing nets (`set_disjunctive_family`)
