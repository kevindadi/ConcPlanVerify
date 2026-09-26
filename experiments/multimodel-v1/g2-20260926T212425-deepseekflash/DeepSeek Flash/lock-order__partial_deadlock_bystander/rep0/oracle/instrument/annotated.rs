mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", 0));
    let b = Arc::new(Mutex::new_named("b_mutex0", 0));
    let sa = Semaphore::new_named("sa_semaphore0", 0);
    let sb = Semaphore::new_named("sb_semaphore0", 0);
    let flag = Arc::new(Mutex::new_named("flag_mutex0", 0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let sa1 = Arc::clone(&sa);
    let sb1 = Arc::clone(&sb);
    let flag1 = Arc::clone(&flag);

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let sa2 = Arc::clone(&sa);
    let sb2 = Arc::clone(&sb);
    let flag2 = Arc::clone(&flag);

    let bystander_flag = Arc::clone(&flag);

    let worker_a = cir_trace::spawn("worker_a", move || {
        let pa = sa1.acquire();
        {
            let _ga = a1.lock().unwrap();
            pa.release();
            sb1.release();
            let _gb = b1.lock().unwrap();
            let mut f = flag1.lock().unwrap();
            *f += 1;
        }
    });

    let worker_b = cir_trace::spawn("worker_b", move || {
        let pb = sb2.acquire();
        {
            let _gb = b2.lock().unwrap();
            pb.release();
            sa2.release();
            let _ga = a2.lock().unwrap();
            let mut f = flag2.lock().unwrap();
            *f += 1;
        }
    });

    let bystander = cir_trace::spawn("yield_now", move || loop {
        let mut f = bystander_flag.lock().unwrap();
        *f += 0;
        drop(f);
        thread::yield_now();
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();

    let _f = flag.lock().unwrap();
    println!("DONE a=1 b=1");

    drop(bystander);
 cir_trace::finish();}
