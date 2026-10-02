mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn t1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    // s1: mutex_lock main::a
    let mut guard_a = a.lock().unwrap();
    // s2: mutex_lock main::b
    let mut guard_b = b.lock().unwrap();
    // s3: work = 1 (critical work while holding both locks)
    let work = 1;
    let _ = work;
    // s4: mutex_unlock main::b
    drop(guard_b);
    // s5: mutex_unlock main::a
    drop(guard_a);
    // s6: return
}

fn t2(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    // s1: mutex_lock main::a
    let mut guard_a = a.lock().unwrap();
    // s2: mutex_lock main::b
    let mut guard_b = b.lock().unwrap();
    // s3: work = 1 (critical work while holding both locks)
    let work = 1;
    let _ = work;
    // s4: mutex_unlock main::b
    drop(guard_b);
    // s5: mutex_unlock main::a
    drop(guard_a);
    // s6: return
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#890", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#928", ()));

    // s1: spawn main::t1
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1#1036", move || t1(&a1, &b1));

    // s2: spawn main::t2
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2#1176", move || t2(&a2, &b2));

    // s3: join h1
    h1.join().unwrap();
    // s4: join h2
    h2.join().unwrap();
    // s5: return — print the required terminal line
    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
