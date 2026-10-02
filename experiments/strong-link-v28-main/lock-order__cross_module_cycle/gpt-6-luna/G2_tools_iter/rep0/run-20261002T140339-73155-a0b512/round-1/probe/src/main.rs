mod first {
    use std::sync::Mutex;

    // The first module owns resource a.
    #[allow(non_upper_case_globals)]
    pub static a: Mutex<()> = Mutex::new(());

    pub fn t1() {
        // t1 also depends on resource b, owned by the second module.
        let guard_a = a.lock().unwrap();
        let guard_b = crate::second::b.lock().unwrap();

        // Work while holding both resources.
        drop(guard_b);
        drop(guard_a);
    }
}

mod second {
    use std::sync::Mutex;

    // The second module owns resource b.
    #[allow(non_upper_case_globals)]
    pub static b: Mutex<()> = Mutex::new(());

    pub fn t2() {
        // t2 also depends on resource a, owned by the first module.
        // Both tasks acquire resources in the same order: a, then b.
        let guard_a = crate::first::a.lock().unwrap();
        let guard_b = b.lock().unwrap();

        // Work while holding both resources.
        drop(guard_b);
        drop(guard_a);
    }
}

fn main() {
    let t1 = std::thread::spawn(first::t1);
    let t2 = std::thread::spawn(second::t2);

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
}
