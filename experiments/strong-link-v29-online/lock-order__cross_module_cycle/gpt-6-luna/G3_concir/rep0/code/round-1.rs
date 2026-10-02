use std::sync::Mutex;
use std::thread;

static a: Mutex<()> = Mutex::new(());

mod other {
    use std::sync::Mutex;

    pub static b: Mutex<()> = Mutex::new(());

    pub fn t2(a: &'static Mutex<()>, b: &'static Mutex<()>) {
        let a_guard = a.lock().unwrap();
        let b_guard = b.lock().unwrap();

        let mut work = 0;
        work = 1;

        drop(b_guard);
        drop(a_guard);
    }
}

fn t1(a: &'static Mutex<()>, b: &'static Mutex<()>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    let mut work = 0;
    work = 1;

    drop(b_guard);
    drop(a_guard);
}

fn main() {
    let t1_handle = thread::spawn(|| t1(&a, &other::b));
    let t2_handle = thread::spawn(|| other::t2(&a, &other::b));

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE done=1");
}
