mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// R1: two modules, each owning one shared resource.
mod module_a {
    use std::sync::{Arc};

    // Resource owned by module A.
    pub struct ResourceA {
        pub value: i32,
    }

    // R2/R3/R4: task in module A needs both resources.
    // R5: lock ordering is enforced by always acquiring A then B.
    pub fn task_a(
        res_a: Arc<Mutex<ResourceA>>,
        res_b: Arc<Mutex<super::module_b::ResourceB>>,
    ) {
        // Acquire in a fixed global order: A first, then B.
        let mut a = res_a.lock().unwrap();
        let mut b = res_b.lock().unwrap();

        // R4: hold both resources while performing work.
        a.value += 1;
        b.value += 1;

        // R6: resources are released when guards drop at end of scope.
        drop(b);
        drop(a);
    }
}

mod module_b {
    use std::sync::{Arc};

    // Resource owned by module B.
    pub struct ResourceB {
        pub value: i32,
    }

    // R2/R3/R4: task in module B needs both resources.
    // R5: same fixed lock order (A then B) prevents deadlock.
    pub fn task_b(
        res_a: Arc<Mutex<super::module_a::ResourceA>>,
        res_b: Arc<Mutex<ResourceB>>,
    ) {
        // Acquire in the same fixed global order: A first, then B.
        let mut a = res_a.lock().unwrap();
        let mut b = res_b.lock().unwrap();

        // R4: hold both resources while performing work.
        a.value += 1;
        b.value += 1;

        // R6: resources are released when guards drop at end of scope.
        drop(b);
        drop(a);
    }
}

fn main() { cir_trace::init();
    // Shared resources.
    let res_a = Arc::new(Mutex::new_named("res_a_mutex0", module_a::ResourceA { value: 0 }));
    let res_b = Arc::new(Mutex::new_named("res_b_mutex0", module_b::ResourceB { value: 0 }));

    // R7: starting thread launches both tasks and waits for both.
    let a1 = Arc::clone(&res_a);
    let b1 = Arc::clone(&res_b);
    let handle_a = cir_trace::spawn("handle_a", move || {
        module_a::task_a(a1, b1);
    });

    let a2 = Arc::clone(&res_a);
    let b2 = Arc::clone(&res_b);
    let handle_b = cir_trace::spawn("handle_b", move || {
        module_b::task_b(a2, b2);
    });

    // R7/R8: wait for both tasks to finish.
    handle_a.join().unwrap();
    handle_b.join().unwrap();

    // R10: print exactly the required line.
    println!("DONE done=1");
 cir_trace::finish();}
