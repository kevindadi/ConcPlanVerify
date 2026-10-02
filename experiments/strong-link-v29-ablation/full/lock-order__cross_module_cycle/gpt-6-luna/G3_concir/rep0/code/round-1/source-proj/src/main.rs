use std::sync::Mutex;
use std::thread;

pub(crate) static a: Mutex<()> = Mutex::new(());

fn t1() {
    let a_guard = a.lock().unwrap();
    let b_guard = other::b.lock().unwrap();
    let work = 1;
    let _ = work;
    drop(b_guard);
    drop(a_guard);
}

mod other {
    use std::sync::Mutex;

    pub(crate) static b: Mutex<()> = Mutex::new(());

    pub(crate) fn t2() {
        let a_guard = crate::a.lock().unwrap();
        let b_guard = b.lock().unwrap();
        let work = 1;
        let _ = work;
        drop(b_guard);
        drop(a_guard);
    }
}

fn main() {
    let t1_handle = thread::spawn(|| t1());
    let t2_handle = thread::spawn(|| other::t2());

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE done=1");
}
