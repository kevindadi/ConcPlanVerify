use std::sync::Mutex;
use std::thread;

#[allow(non_upper_case_globals)]
mod other {
    use std::sync::Mutex;

    #[allow(non_upper_case_globals)]
    pub static b: Mutex<()> = Mutex::new(());

    pub fn t2() {
        use crate::main::a;

        let a_guard = a.lock().unwrap();
        let b_guard = b.lock().unwrap();
        let work = 1;
        let _ = work;
        drop(b_guard);
        drop(a_guard);
    }
}

#[allow(non_upper_case_globals)]
mod main {
    use std::sync::Mutex;
    use std::thread;

    #[allow(non_upper_case_globals)]
    pub static a: Mutex<()> = Mutex::new(());

    pub fn t1() {
        use crate::other::b;

        let a_guard = a.lock().unwrap();
        let b_guard = b.lock().unwrap();
        let work = 1;
        let _ = work;
        drop(b_guard);
        drop(a_guard);
    }

    pub fn main() {
        use crate::other::t2;

        thread::scope(|s| {
            s.spawn(t1);
            s.spawn(t2);
        });
        println!("DONE done=1");
    }
}

fn main() {
    main::main();
}
