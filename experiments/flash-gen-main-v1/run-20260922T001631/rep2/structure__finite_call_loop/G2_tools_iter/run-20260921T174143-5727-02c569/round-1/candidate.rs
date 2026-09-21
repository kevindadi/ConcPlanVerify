use std::thread;

fn auxiliary() {
    // Auxiliary routine: does some work and returns.
    // No shared state with the main task.
    let mut x = 0u64;
    for i in 0..1000 {
        x = x.wrapping_add(i);
    }
    // Prevent optimization from removing the work.
    std::hint::black_box(x);
}

fn main() {
    // R1: main task calls auxiliary, then begins the same call sequence again.
    // R3: each auxiliary call runs to completion before the next call starts.
    // R2: no shared state between main and auxiliary.
    // R4: every schedule terminates (join ensures completion).
    // R5: print exactly "DONE done=1".

    let handle1 = thread::spawn(auxiliary);
    handle1.join().unwrap();

    let handle2 = thread::spawn(auxiliary);
    handle2.join().unwrap();

    println!("DONE done=1");
}
