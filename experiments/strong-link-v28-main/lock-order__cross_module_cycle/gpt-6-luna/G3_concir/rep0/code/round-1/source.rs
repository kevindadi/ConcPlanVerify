use std::sync::{Arc, Mutex};
use std::thread;

mod cir_modules {
    pub mod main {
        use std::sync::{Arc, Mutex};
        use std::thread;

        pub fn main(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
            let a1 = Arc::clone(&a);
            let b1 = Arc::clone(&b);
            let t1_handle = thread::spawn(move || t1(a1, b1));

            let a2 = Arc::clone(&a);
            let b2 = Arc::clone(&b);
            let t2_handle = thread::spawn(move || super::other::t2(a2, b2));

            t1_handle.join().unwrap();
            t2_handle.join().unwrap();
        }

        pub fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
            let guard_a = a.lock().unwrap();
            let guard_b = b.lock().unwrap();
            drop(guard_b);
            drop(guard_a);
        }
    }

    pub mod other {
        use std::sync::{Arc, Mutex};

        pub fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
            let guard_a = a.lock().unwrap();
            let guard_b = b.lock().unwrap();
            drop(guard_b);
            drop(guard_a);
        }
    }
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    cir_modules::main::main(a, b);
    println!("DONE done=1");
}
