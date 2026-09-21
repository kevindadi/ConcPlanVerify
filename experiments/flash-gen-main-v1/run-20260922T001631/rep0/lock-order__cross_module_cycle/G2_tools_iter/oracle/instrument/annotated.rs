mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Module 1 owns the first shared resource.
mod module1 {
    use std::sync::{Arc};

    pub struct Resource1 {
        pub value: i32,
    }

    pub fn task1(
        res1: Arc<Mutex<Resource1>>,
        res2: Arc<Mutex<super::module2::Resource2>>,
        done: Arc<Mutex<i32>>,
    ) {
        // Acquire both resources in a consistent global order to avoid deadlock.
        let mut r1 = res1.lock().unwrap();
        let mut r2 = res2.lock().unwrap();

        // Hold both resources at the same time while performing work.
        r1.value += 1;
        r2.value += 1;

        // Release resources before finishing.
        drop(r2);
        drop(r1);

        let mut d = done.lock().unwrap();
        *d += 1;
    }
}

// Module 2 owns the second shared resource.
mod module2 {
    use std::sync::{Arc};

    pub struct Resource2 {
        pub value: i32,
    }

    pub fn task2(
        res1: Arc<Mutex<super::module1::Resource1>>,
        res2: Arc<Mutex<Resource2>>,
        done: Arc<Mutex<i32>>,
    ) {
        // Acquire both resources in the same global order as task1.
        let mut r1 = res1.lock().unwrap();
        let mut r2 = res2.lock().unwrap();

        // Hold both resources at the same time while performing work.
        r1.value += 1;
        r2.value += 1;

        // Release resources before finishing.
        drop(r2);
        drop(r1);

        let mut d = done.lock().unwrap();
        *d += 1;
    }
}

fn main() { cir_trace::init();
    let res1 = Arc::new(Mutex::new_named("res1_mutex0", module1::Resource1 { value: 0 }));
    let res2 = Arc::new(Mutex::new_named("res2_mutex0", module2::Resource2 { value: 0 }));
    let done = Arc::new(Mutex::new_named("done_mutex0", 0));

    let r1a = Arc::clone(&res1);
    let r2a = Arc::clone(&res2);
    let da = Arc::clone(&done);

    let r1b = Arc::clone(&res1);
    let r2b = Arc::clone(&res2);
    let db = Arc::clone(&done);

    let t1 = cir_trace::spawn("t1", move || {
        module1::task1(r1a, r2a, da);
    });

    let t2 = cir_trace::spawn("t2", move || {
        module2::task2(r1b, r2b, db);
    });

    t1.join().unwrap();
    t2.join().unwrap();

    let d = done.lock().unwrap();
    println!("DONE done={}", *d);
 cir_trace::finish();}
