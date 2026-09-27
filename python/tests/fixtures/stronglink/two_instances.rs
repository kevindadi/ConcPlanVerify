use std::sync::{Arc, Mutex};
struct S { m: Mutex<i32> }
fn main() {
    let a = Arc::new(S { m: Mutex::new(0) });
    let b = Arc::new(S { m: Mutex::new(1) });
    let ga = a.m.lock().unwrap();
    drop(ga);
    let gb = b.m.lock().unwrap();
    drop(gb);
}
