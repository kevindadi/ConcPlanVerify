# CVN Specification Reference

> See [`../cvn/README.md`](../cvn/README.md) for the CVN library documentation.

The CVN (Concurrency Verification Net) is a weighted P/T Petri net with global variable guards.
Key types are in `cvn::model`:

- `CvnNet` — the complete net (petgraph-backed)
- `Place` / `PlaceId` / `PlaceKind` — places (Control, Resource, Wait)
- `Transition` / `TransitionId` / `TransitionKind` — transitions with kind labels
- `InputArcData` / `OutputArcData` — arcs with weight, guard, and update
- `BoolExpr` / `Expr` — guard and value expressions
- `Val` / `ConcreteVal` — values (Concrete or Unknown)
- `State` / `Marking` / `VarStore` — runtime state for analysis
- `CvnNetBuilder` — builder API for constructing nets
