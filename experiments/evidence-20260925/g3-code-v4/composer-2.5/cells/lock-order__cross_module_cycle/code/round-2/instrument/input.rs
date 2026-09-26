use std::sync::Mutex;
use std::thread;

mod other {
    use std::sync::Mutex;

    pub static b: Mutex<()> = Mutex::new(());

    pub fn t2() {
        let _guard_a = crate::main::a.lock().unwrap();
        let _guard_b = b.lock().unwrap();
        drop(_guard_b);
        drop(_guard_a);
    }
}

mod main {
    use std::sync::Mutex;
    use std::thread;

    pub static a: Mutex<()> = Mutex::new(());

    pub fn t1() {
        let _guard_a = a.lock().unwrap();
        let _guard_b = crate::other::b.lock().unwrap();
        drop(_guard_b);
        drop(_guard_a);
    }

    pub fn main() {
        let handle_t1 = thread::spawn(t1);
        let handle_t2 = thread::spawn(crate::other::t2);

        handle_t1.join().unwrap();
        handle_t2.join().unwrap();
    }
}

fn main() {
    main::main();
    println!("DONE done=1");
}
