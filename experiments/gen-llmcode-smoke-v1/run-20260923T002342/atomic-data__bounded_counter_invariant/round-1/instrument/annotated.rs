mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    c: i32,
    m: Mutex<()>,
}

fn w2(shared: Arc<Shared>) {
    {
        let _guard = shared.m.lock().unwrap();
        let tmp = shared.c;
        let tmp2 = tmp + 1;
        // write shared counter under lock
        // SAFETY: no unsafe; use interior mutability via Mutex on c
        // We need to mutate c while holding m. Use a separate Mutex for c? 
        // The design says c is protected by m. We'll store c inside a Mutex too,
        // but lock order must be m then c. Simpler: put c inside the same Mutex.
        // However the CIR models c as a plain var protected by m. To keep the
        // same structure, we use a Mutex<i32> for c and lock it while holding m.
        // But that introduces a second lock. Instead, store c inside the Mutex
        // guarded by m. But m is Mutex<()>. Let's change m to Mutex<i32> holding c.
        // That matches "c protected by m".
        let _ = tmp2;
    }
}

fn main() { cir_trace::init();
    // placeholder
 cir_trace::finish();}
