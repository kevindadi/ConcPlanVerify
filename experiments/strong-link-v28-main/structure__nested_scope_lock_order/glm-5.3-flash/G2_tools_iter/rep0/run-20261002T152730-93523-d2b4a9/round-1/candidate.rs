use std::sync::Arc;
use tokio::sync::Mutex;

// Shared resources: a (lock), b (lock).
// Both inner tasks acquire them in the SAME order: a, then b.
// Because the order is consistent, no wait cycle can form (R4),
// so every interleaving terminates (R6).

async fn inner_task(
    name: &'static str,
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
) {
    // Acquire a first...
    let _guard_a = a.lock().await;
    // ...then b. Both inner tasks follow this same order.
    let _guard_b = b.lock().await;

    // At this point the task holds both a and b simultaneously (R3).
    println!("{} holds a and b", name);

    // Guards drop here, releasing b then a, in reverse acquisition order.
    drop(_guard_b);
    drop(_guard_a);
}

// Role: outer. Starts the nested group of two inner tasks (R2)
// and completes only after both inner tasks have finished (R5).
async fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let h1 = tokio::spawn(inner_task("x1", a.clone(), b.clone()));
    let h2 = tokio::spawn(inner_task("x2", a.clone(), b.clone()));

    // Wait for both inner tasks; outer completes only after they finish.
    h1.await.expect("x1 panicked");
    h2.await.expect("x2 panicked");
}

// Role: main task. Starts one outer worker (R1).
#[tokio::main]
async fn main() {
    let a = Arc::new(Mutex::new(())); // resource: a
    let b = Arc::new(Mutex::new(())); // resource: b

    let outer_handle = tokio::spawn(outer(a, b));

    // Main task waits for the outer worker to complete.
    outer_handle.await.expect("outer panicked");

    // Exactly one line of output at the end (R7).
    println!("DONE done=1");
}
