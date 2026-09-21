mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Module A owns resource A
mod module_a {
    use std::sync::{Arc};

    pub struct ResourceA {
        pub value: i32,
    }

    pub fn task_a(res_a: Arc<Mutex<ResourceA>>, res_b: Arc<Mutex<super::module_b::ResourceB>>) {
        // Acquire both resources in a consistent global order to avoid deadlock.
        // Order: A then B.
        let mut a = res_a.lock().unwrap();
        let mut b = res_b.lock().unwrap();

        // Hold both resources at the same time while performing work.
        a.value += 1;
        b.value += 1;

        // Resources are released automatically when guards go out of scope.
    }
}

// Module B owns resource B
mod module_b {
    use std::sync::{Arc};

    pub struct ResourceB {
        pub value: i32,
    }

    pub fn task_b(res_a: Arc<Mutex<super::module_a::ResourceA>>, res_b: Arc<Mutex<ResourceB>>) {
        // Acquire both resources in the same global order: A then B.
        let mut a = res_a.lock().unwrap();
        let mut b = res_b.lock().unwrap();

        a.value += 1;
        b.value += 1;
    }
}

fn main() { cir_trace::init();
    let res_a = Arc::new(Mutex::new_named("res_a_mutex0", module_a::ResourceA { value: 0 }));
    let res_b = Arc::new(Mutex::new_named("res_b_mutex0", module_b::ResourceB { value: 0 }));

    let a1 = Arc::clone(&res_a);
    let b1 = Arc::clone(&res_b);
    let a2 = Arc::clone(&res_a);
    let b2 = Arc::clone(&res_b);

    let t1 = cir_trace::spawn("t1", move || {
        module_a::task_a(a1, b1);
    });

    let t2 = cir_trace::spawn("t2", move || {
        module_b::task_b(a2, b2);
    });

    t1.join().unwrap();
    t2.join().unwrap();

    let done = {
        let a = res_a.lock().unwrap();
        let b = res_b.lock().unwrap();
        a.value == 2 && b.value == 2
    };

    println!("DONE done={}", if done { 1 } else { 0 });
 cir_trace::finish();}
