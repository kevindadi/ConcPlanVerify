mod cir_trace;
use std::thread;

fn worker() {
    // CIR main::worker body: s1 return
}

fn main() { cir_trace::init();
    let mut done: i32 = 0;

    // s1: spawn main::worker -> h1
    let h1 = cir_trace::spawn("worker#164", || worker());
    // s2: join h1
    h1.join().unwrap();
    // s3: done = 1
    done = 1;

    // s4: spawn main::worker -> h2
    let h2 = cir_trace::spawn("worker#319", || worker());
    // s5: join h2
    h2.join().unwrap();
    // s6: done = 1
    done = 1;

    // s7: return (terminal line)
    println!("DONE done={}", done);
 cir_trace::finish();}
