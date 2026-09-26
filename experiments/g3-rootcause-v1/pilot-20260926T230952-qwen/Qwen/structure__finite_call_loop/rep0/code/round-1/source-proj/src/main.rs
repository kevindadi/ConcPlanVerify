use std::sync::atomic::{AtomicUsize, Ordering};

static DONE: AtomicUsize = AtomicUsize::new(0);

fn helper() {
    // no operations
}

fn main() {
    helper();
    helper();
    DONE.store(1, Ordering::SeqCst);
    println!("DONE done={}", DONE.load(Ordering::SeqCst));
}
