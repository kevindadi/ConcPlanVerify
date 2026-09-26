use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

/// One activation of a worker role.
///
/// R4: `acquire()` blocks until the single shared permit becomes available.
/// R2: the activation holds the permit while it does its work and releases
///     it afterwards (dropping the permit releases it exactly once).
fn activation(s: Arc<Semaphore>) {
    let permit = s.acquire();

    // Work performed while holding the permit.
    let mut acc: u64 = 0;
    for i in 0..1_000u64 {
        acc = acc.wrapping_mul(31).wrapping_add(i);
    }
    std::hint::black_box(acc);

    drop(permit); // release the permit after the work is done
}

/// A worker role: runs up to two activations concurrently (R1), then waits
/// for both of them to finish (R5, R6).
fn role(s: &Arc<Semaphore>) {
    let mut handles = Vec::new();
    for _ in 0..2 {
        let sc = Arc::clone(s);
        handles.push(thread::spawn(move || activation(sc)));
    }
    for h in handles {
        h.join().unwrap();
    }
}

fn w1(s: Arc<Semaphore>) {
    role(&s);
}

fn w2(s: Arc<Semaphore>) {
    role(&s);
}

fn w3(s: Arc<Semaphore>) {
    role(&s);
}

fn main() {
    // R1/R3: a single permit shared by all activations of all three roles,
    // so at most one activation can hold it at any moment.
    let s = Semaphore::new(1);

    let h1 = thread::spawn({
        let s = Arc::clone(&s);
        move || w1(s)
    });
    let h2 = thread::spawn({
        let s = Arc::clone(&s);
        move || w2(s)
    });
    let h3 = thread::spawn({
        let s = Arc::clone(&s);
        move || w3(s)
    });

    // R5/R6: wait for all three roles (and therefore every activation).
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    // R7
    println!("DONE done=1");
}
