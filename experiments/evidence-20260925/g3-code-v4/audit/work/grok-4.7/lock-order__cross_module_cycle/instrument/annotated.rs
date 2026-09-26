mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
#[allow(non_upper_case_globals)]
mod other {
    use std::sync::LazyLock;
    use crate::cir_trace::sync::{Mutex};

    pub static b: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new_named("b_mutex0", ()));

    pub fn t2() {
        use crate::main::a;

        let a_guard = a.lock().unwrap();
        let b_guard = b.lock().unwrap();
        let mut work = 0;
        work = 1;
        let _ = work;
        drop(b_guard);
        drop(a_guard);
    }
}

#[allow(non_upper_case_globals)]
mod main {
    use std::sync::LazyLock;
    use crate::cir_trace::sync::{Mutex};
    use std::thread;

    pub static a: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new_named("a_mutex0", ()));

    pub fn t1() {
        use crate::other::b;

        let a_guard = a.lock().unwrap();
        let b_guard = b.lock().unwrap();
        let mut work = 0;
        work = 1;
        let _ = work;
        drop(b_guard);
        drop(a_guard);
    }

    pub fn main() {
        use crate::other::t2;

        thread::scope(|s| {
            s.spawn(t1);
            s.spawn(t2);
        });
        println!("DONE done=1");
    }
}

fn main() { cir_trace::init();
    main::main();
 cir_trace::finish();}
