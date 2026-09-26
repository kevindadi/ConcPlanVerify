mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
mod r#main {
    use crate::cir_trace::sync::{Mutex};
    use std::thread;

    pub static a: Mutex<()> = Mutex::new(());

    pub fn t1() {
        let _ga = a.lock().unwrap();
        let _gb = super::other::b.lock().unwrap();
    }

    pub fn main() {
        thread::scope(|s| {
            s.spawn(t1);
            s.spawn(super::other::t2);
        });
    }
}

mod other {
    use crate::cir_trace::sync::{Mutex};

    pub static b: Mutex<()> = Mutex::new(());

    pub fn t2() {
        let _ga = super::r#main::a.lock().unwrap();
        let _gb = b.lock().unwrap();
    }
}

fn main() { cir_trace::init();
    r#main::main();
    println!("DONE done=1");
 cir_trace::finish();}
