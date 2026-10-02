use std::thread;

// CIR function main::worker — no shared work, no shared resources.
fn worker() {
}

fn main() {
    // CIR s1: spawn main::worker as w1, calling the named entry.
    let w1 = thread::spawn(|| worker());
    // CIR s2: join w1 exactly once.
    w1.join().expect("worker w1 panicked");

    // CIR s3: spawn main::worker as w2, calling the named entry.
    let w2 = thread::spawn(|| worker());
    // CIR s4: join w2 exactly once.
    w2.join().expect("worker w2 panicked");

    // CIR s5: return after printing the required terminal line.
    println!("DONE done=1");
}
