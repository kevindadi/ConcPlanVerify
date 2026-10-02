use std::sync::{Arc, Mutex, Condvar};
use std::thread;

fn main() {
    let ready = Arc::new((Mutex::new(false), Condvar::new()));

    let ready_clone1 = Arc::clone(&ready);
    let notifier_handle = thread::spawn(move || {
        let (lock, cvar) = &*ready_clone1;
        let mut flag = lock.lock().unwrap();
        *flag = true;
        cvar.notify_all();
        // Lock is released when `flag` goes out of scope
    });

    let ready_clone2 = Arc::clone(&ready);
    let waiter_handle = thread::spawn(move || {
        let (lock, cvar) = &*ready_clone2;
        let mut flag = lock.lock().unwrap();
        while !*flag {
            flag = cvar.wait(flag).unwrap();
        }
        // Lock is released when `flag` goes out of scope
    });

    notifier_handle.join().unwrap();
    waiter_handle.join().unwrap();

    println!("DONE ready=true");
}
