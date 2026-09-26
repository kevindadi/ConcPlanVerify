use std::sync::{Arc, Mutex};
use std::thread;

mod other {
    use std::sync::{Arc, Mutex};

    pub fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
        let _guard_a = a.lock().unwrap();
        let _guard_b = b.lock().unwrap();
        drop(_guard_b);
        drop(_guard_a);
    }
}

mod main {
    use std::sync::{Arc, Mutex};
    use std::thread;

    pub fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
        let _guard_a = a.lock().unwrap();
        let _guard_b = b.lock().unwrap();
        drop(_guard_b);
        drop(_guard_a);
    }

    pub fn main() {
        let a = Arc::new(Mutex::new(()));
        let b = Arc::new(Mutex::new(()));

        let a_t1 = Arc::clone(&a);
        let b_t1 = Arc::clone(&b);
        let a_t2 = Arc::clone(&a);
        let b_t2 = Arc::clone(&b);

        let handle_t1 = thread::spawn(move || t1(a_t1, b_t1));
        let handle_t2 = thread::spawn(move || crate::other::t2(a_t2, b_t2));

        handle_t1.join().unwrap();
        handle_t2.join().unwrap();
    }
}

fn main() {
    main::main();
    println!("DONE done=1");
}
