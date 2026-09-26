mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
mod cir_trace;

mod r#main {
    use std::sync::LazyLock;
    use std::thread;

    use crate::cir_trace::sync::Mutex;

    pub static a: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    pub fn t1() {
        let ga = a.lock().unwrap();
        let gb = super::other::b.lock().unwrap();
        drop(gb);
        drop(ga);
    }

    pub fn main() {
        thread::scope(|s| {
            s.spawn(t1);
            s.spawn(super::other::t2);
        });
    }
}

mod other {
    use std::sync::LazyLock;

    use crate::cir_trace::sync::Mutex;

    pub static b: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    pub fn t2() {
        let ga = super::r#main::a.lock().unwrap();
        let gb = b.lock().unwrap();
        drop(gb);
        drop(ga);
    }
}

fn main() { cir_trace::init();
    r#main::main();
    println!("DONE done=1");
 cir_trace::finish();}
