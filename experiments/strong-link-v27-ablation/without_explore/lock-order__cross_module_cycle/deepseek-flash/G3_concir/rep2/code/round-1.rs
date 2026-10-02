mod modA {
    use std::sync::Mutex;

    // Resource owned by module modA.
    pub static a: Mutex<()> = Mutex::new(());

    /// Task t1: locks a, then b (the resource owned by modB), uses both,
    /// then releases each resource before returning.
    pub fn t1() {
        let guard_a = a.lock().unwrap();
        let guard_b = crate::modB::b.lock().unwrap();

        let x_init: i32 = 0;
        let x: i32 = 1;
        let _ = x_init;
        let _ = x;

        drop(guard_b);
        drop(guard_a);
    }

    /// Entry scope: launches both tasks and waits for both to finish.
    pub fn main() {
        let h1 = std::thread::spawn(|| {
            modA::t1();
        });
        let h2 = std::thread::spawn(|| {
            crate::modB::t2();
        });

        h1.join().unwrap();
        h2.join().unwrap();

        println!("DONE done=1");
    }
}

mod modB {
    use std::sync::Mutex;

    // Resource owned by module modB.
    pub static b: Mutex<()> = Mutex::new(());

    /// Task t2: locks a (the resource owned by modA), then b, uses both,
    /// then releases each resource before returning.
    pub fn t2() {
        let guard_a = crate::modA::a.lock().unwrap();
        let guard_b = b.lock().unwrap();

        let y_init: i32 = 0;
        let y: i32 = 1;
        let _ = y_init;
        let _ = y;

        drop(guard_b);
        drop(guard_a);
    }
}

fn main() {
    modA::main();
}
