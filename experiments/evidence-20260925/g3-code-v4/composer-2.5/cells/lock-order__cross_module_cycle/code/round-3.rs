use std::sync::Mutex;
use std::thread;

mod main {
    pub mod other {
        pub static b: Mutex<()> = Mutex::new(());

        pub fn t2() {
            let _guard_a = super::a.lock().unwrap();
            let _guard_b = b.lock().unwrap();
            drop(_guard_b);
            drop(_guard_a);
        }
    }

    pub static a: Mutex<()> = Mutex::new(());

    pub fn t1() {
        let _guard_a = a.lock().unwrap();
        let _guard_b = other::b.lock().unwrap();
        drop(_guard_b);
        drop(_guard_a);
    }

    pub fn main() {
        let handle_t1 = thread::spawn(t1);
        let handle_t2 = thread::spawn(other::t2);
        handle_t1.join().unwrap();
        handle_t2.join().unwrap();
    }
}

mod other {
    pub use crate::main::other::b;
    pub use crate::main::other::t2;
}

fn main() {
    main::main();
    println!("DONE done=1");
}
