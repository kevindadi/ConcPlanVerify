mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, sem: Arc<concir_sync::Semaphore>) {
    let _permit = sem.acquire();
    let _la = a.lock().unwrap();
    let _lb = b.lock().unwrap();
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>, sem: Arc<concir_sync::Semaphore>) {
    let _permit = sem.acquire();
    let _lb = b.lock().unwrap();
    let _lc = c.lock().unwrap();
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>, sem: Arc<concir_sync::Semaphore>) {
    let _permit = sem.acquire();
    let _la = a.lock().unwrap();
    let _lc = c.lock().unwrap();
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));
    let sem = Semaphore::new_named("sem_semaphore0", 1);

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sem = Arc::clone(&sem);
        cir_trace::spawn("t1", move || t1(a, b, sem))
    };
    let h2 = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        let sem = Arc::clone(&sem);
        cir_trace::spawn("t2", move || t2(b, c, sem))
    };
    let h3 = {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        let sem = Arc::clone(&sem);
        cir_trace::spawn("t3", move || t3(a, c, sem))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
