mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

use concir_sync::Semaphore;

// main::bystander: an independent task that keeps making progress and never
// finishes on its own.
fn bystander() {
    loop {
        std::hint::spin_loop();
    }
}

// Leak a semaphore so the permit token it hands out can live in a worker
// thread for the whole program.
fn static_semaphore(s: Arc<Semaphore>) -> &'static Semaphore {
    let leaked: &'static Arc<Semaphore> = Box::leak(Box::new(s));
    &**leaked
}

fn main() { cir_trace::init();
    // Shared resources, exactly the ones named in the design.
    let a = Arc::new(Mutex::new_named("a_mutex0#602", 0i32)); // lock a, protects `flag`
    let b = Arc::new(Mutex::new_named("b_mutex0#669", ()));   // lock b
    let sa = static_semaphore(Semaphore::new_named("sa_semaphore0#732", 1)); // counting permits sa
    let sb = static_semaphore(Semaphore::new_named("sb_semaphore0#805", 1)); // counting permits sb

    // Take the two handshake tokens out of the semaphores. Afterwards sa and sb
    // hold no free permit, so a and b must pass the tokens to each other.
    let permit_sa = sa.acquire();
    let permit_sb = sb.acquire();

    // main: spawn bystander, a, b. The bystander is detached and never joined.
    let _hbystander = cir_trace::spawn("_hbystander#1165", bystander);

    let a_ha = Arc::clone(&a);
    let b_ha = Arc::clone(&b);
    let ha = cir_trace::spawn("ha#1267", move || {
        let guard_a = a_ha.lock().unwrap();     // s1: lock a
        permit_sa.release();                    // s2: release sa
        drop(guard_a);                          // s3: unlock a
        let _permit_sb = sb.acquire();          // s4: acquire sb
        let mut guard_a = a_ha.lock().unwrap(); // s5: lock a
        let guard_b = b_ha.lock().unwrap();     // s6: lock b
        *guard_a = 1;                           // s7: flag = 1
        drop(guard_b);                          // s8: unlock b
        drop(guard_a);                          // s9: unlock a
    });

    let a_hb = Arc::clone(&a);
    let b_hb = Arc::clone(&b);
    let hb = cir_trace::spawn("hb#1949", move || {
        let guard_b = b_hb.lock().unwrap();     // s1: lock b
        permit_sb.release();                    // s2: release sb
        drop(guard_b);                          // s3: unlock b
        let _permit_sa = sa.acquire();          // s4: acquire sa
        let mut guard_a = a_hb.lock().unwrap(); // s5: lock a
        let guard_b = b_hb.lock().unwrap();     // s6: lock b
        *guard_a = 1;                           // s7: flag = 1
        drop(guard_b);                          // s8: unlock b
        drop(guard_a);                          // s9: unlock a
    });

    // main: join a, then b.
    ha.join().unwrap();
    hb.join().unwrap();

    println!("DONE a=1 b=1");
 cir_trace::finish();}
