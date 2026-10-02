// SHADOW control: two distinct `AtomicUsize` objects both named `c` in different
// functions. There is no unique construction for the name, so the value identity
// is unresolved and must stay unsupported (never guessed by name).
use std::sync::atomic::{AtomicUsize, Ordering};

fn a() {
    let c = AtomicUsize::new(0);
    c.store(2, Ordering::SeqCst);
}
fn b() {
    let c = AtomicUsize::new(5);
    c.store(9, Ordering::SeqCst);
}
fn main() {
    a();
    b();
    println!("DONE done=1");
}
