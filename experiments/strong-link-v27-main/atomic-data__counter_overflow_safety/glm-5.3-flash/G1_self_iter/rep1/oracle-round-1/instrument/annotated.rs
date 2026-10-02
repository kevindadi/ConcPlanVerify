mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

/// Shared counter `c`.
/// Declared range: 0..=2 (fits in u8). Initial value: 0. (R2)
struct C(u8);

/// Worker body shared by w1 and w2.
/// All reads and writes of `c` occur while holding the lock `m`. (R4)
fn worker(_name: &'static str, m: &Arc<Mutex<C>>) {
    let mut g = m.lock().unwrap();
    // R3/R5: increment only if the result stays within the limit.
    // The check and the write are one critical section, so no
    // interleaving can push `c` above 1.
    if g.0 < 1 {
        g.0 += 1;
    }
    // If `c` is already 1, adding one would exceed the limit,
    // so this worker leaves `c` unchanged and finishes. (R6)
    // Lock is released here when `g` is dropped.
}

fn main() { cir_trace::init();
    // m: the mutual-exclusion lock guarding c.
    let m: Arc<Mutex<C>> = Arc::new(Mutex::new_named("res_mutex0#838", C(0)));

    // R1: supervising task launches w1 and w2 and waits for both.
    let h1 = {
        let m = Arc::clone(&m);
        thread::Builder::new()
            .name("w1".to_string())
            .spawn(move || worker("w1", &m))
            .unwrap()
    };
    let h2 = {
        let m = Arc::clone(&m);
        thread::Builder::new()
            .name("w2".to_string())
            .spawn(move || worker("w2", &m))
            .unwrap()
    };

    h1.join().unwrap();
    h2.join().unwrap();

    // Read the final value under the lock (R4).
    let done = m.lock().unwrap().0;

    // R7: exactly one worker increments, so this prints `DONE done=1`.
    println!("DONE done={}", done);
 cir_trace::finish();}
