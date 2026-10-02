use std::sync::Mutex;

mod m1 {
    /// Resource `a`, owned by module m1.
    pub static A: Mutex<()> = Mutex::new(());

    /// Task t1: needs both resources (owned by m1 and m2).
    pub fn t1() {
        // Acquire resources in a single, globally consistent order (a then b)
        // so that no cycle / deadlock is possible.
        let guard_a = A.lock().unwrap();
        let guard_b = crate::m2::B.lock().unwrap();

        // Work happens while both resources are held here.

        // Release the resource owned by the other module, then our own.
        drop(guard_b);
        drop(guard_a);
    }

    /// Starting thread: launches both tasks and waits for both to finish.
    pub fn main() {
        let h1 = std::thread::spawn(t1);
        let h2 = std::thread::spawn(crate::m2::t2);

        h1.join().unwrap();
        h2.join().unwrap();
    }
}

mod m2 {
    use std::sync::Mutex;

    /// Resource `b`, owned by module m2.
    pub static B: Mutex<()> = Mutex::new(());

    /// Task t2: needs both resources (owned by m2 and m1).
    pub fn t2() {
        // Same acquisition order as t1 (a then b): deadlock impossible.
        let guard_a = crate::m1::A.lock().unwrap();
        let guard_b = B.lock().unwrap();

        // Work happens while both resources are held here.

        // Release the resource owned by the other module, then our own.
        drop(guard_b);
        drop(guard_a);
    }
}

fn main() {
    m1::main();
    println!("DONE done=1");
}
