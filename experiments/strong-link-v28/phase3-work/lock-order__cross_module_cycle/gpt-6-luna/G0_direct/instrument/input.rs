use std::sync::Mutex;
use std::thread;

mod module_a {
    use std::sync::Mutex;

    #[allow(non_upper_case_globals)]
    pub static a: Mutex<()> = Mutex::new(());

    pub fn t1() {
        // Both tasks acquire a before b, preventing a circular wait.
        let a_guard = a.lock().unwrap();
        let b_guard = crate::module_b::b.lock().unwrap();

        let _both = (&a_guard, &b_guard);

        drop(b_guard);
        drop(a_guard);
    }
}

mod module_b {
    use std::sync::Mutex;

    #[allow(non_upper_case_globals)]
    pub static b: Mutex<()> = Mutex::new(());

    pub fn t2() {
        // Both tasks acquire a before b, preventing a circular wait.
        let a_guard = crate::module_a::a.lock().unwrap();
        let b_guard = b.lock().unwrap();

        let _both = (&a_guard, &b_guard);

        drop(b_guard);
        drop(a_guard);
    }
}

fn main() {
    let t1 = thread::spawn(module_a::t1);
    let t2 = thread::spawn(module_b::t2);

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
}
