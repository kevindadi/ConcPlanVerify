mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
mod io {
    use std::sync::{Arc};use crate::cir_trace::sync::{Mutex};

    pub struct ProtectedByA {
        pub done_t1: i32,
        pub done_t2: i32,
    }

    pub fn println(a: &Arc<Mutex<ProtectedByA>>) {
        let guard = a.lock().unwrap();
        print!("DONE t1={} t2={}\n", guard.done_t1, guard.done_t2);
    }
}

mod main {
    use super::io::{self, ProtectedByA};
    use std::sync::{Arc};use crate::cir_trace::sync::{Mutex};
    use std::thread;

    pub fn t1(a: Arc<Mutex<ProtectedByA>>, b: Arc<Mutex<()>>) {
        let mut guard_a = a.lock().unwrap();
        let _guard_b = b.lock().unwrap();
        guard_a.done_t1 = 1;
        drop(_guard_b);
        drop(guard_a);
    }

    pub fn t2(a: Arc<Mutex<ProtectedByA>>, b: Arc<Mutex<()>>) {
        let mut guard_a = a.lock().unwrap();
        let _guard_b = b.lock().unwrap();
        guard_a.done_t2 = 1;
        drop(_guard_b);
        drop(guard_a);
    }

    pub fn emit_done(a: &Arc<Mutex<ProtectedByA>>) {
        io::println(a);
    }

    pub fn main() {
        let a = Arc::new(Mutex::new_named("a_mutex0", ProtectedByA {
            done_t1: 0,
            done_t2: 0,
        }));
        let b = Arc::new(Mutex::new_named("b_mutex0", ()));

        let a_t1 = Arc::clone(&a);
        let b_t1 = Arc::clone(&b);
        let handle_t1 = cir_trace::spawn("t1", move || t1(a_t1, b_t1));

        let a_t2 = Arc::clone(&a);
        let b_t2 = Arc::clone(&b);
        let handle_t2 = cir_trace::spawn("t2", move || t2(a_t2, b_t2));

        handle_t1.join().unwrap();
        handle_t2.join().unwrap();

        emit_done(&a);
    }
}

fn main() { cir_trace::init();
    main::main();
 cir_trace::finish();}
