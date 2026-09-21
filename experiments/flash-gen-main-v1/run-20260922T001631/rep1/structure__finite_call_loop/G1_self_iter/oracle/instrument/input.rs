use std::thread;

fn auxiliary() {
    // Auxiliary routine: no shared state, just does some work.
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
    auxiliary();
    auxiliary();

    // R5: print exactly the required line.
    println!("DONE done=1");
}
