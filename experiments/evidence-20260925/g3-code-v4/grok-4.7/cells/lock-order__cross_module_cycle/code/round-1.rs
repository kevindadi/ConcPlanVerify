use std::sync::Mutex;
use std::thread;

#[allow(non_upper_case_globals)]
static a: Mutex<()> = Mutex::new(());

mod other {
    use std::sync::Mutex;

    #[allow(non_upper_case_globals)]
    pub static b: Mutex<()> = Mutex::new(());

    pub fn t2() {
        let mut work = 0;
        let guard_a = crate::a.lock().unwrap();
        let guard_b = b.lock().unwrap();
        work = 1;
        drop(guard_b);
        drop(guard_a);
        let _ = work;
    }
}

fn t1() {
    let mut work = 0;
    let guard_a = a.lock().unwrap();
    let guard_b = other::b.lock().unwrap();
    work = 1;
    drop(guard_b);
    drop(guard_a);
    let _ = work;
}

fn main() {
    let h1 = thread::spawn(t1);
    let h2 = thread::spawn(other::t2);
    h1.join().unwrap();
    h2.join().unwrap();
    println!("DONE done=1");
}
