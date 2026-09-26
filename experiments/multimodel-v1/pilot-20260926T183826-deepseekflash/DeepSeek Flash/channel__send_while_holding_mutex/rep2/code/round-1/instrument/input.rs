use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx1, rx1) = mpsc::sync_channel::<i32>(0);
    let (tx2, rx2) = mpsc::sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let s = thread::spawn(move || {
        let mut v: i32 = 0;
        {
            let _guard = m_s.lock().unwrap();
        }
        tx1.send(1).unwrap();
        v = rx2.recv().unwrap();
        let _ = v;
    });

    let m_r = Arc::clone(&m);
    let r = thread::spawn(move || {
        let mut v: i32 = 0;
        {
            let _guard = m_r.lock().unwrap();
        }
        v = rx1.recv().unwrap();
        tx2.send(1).unwrap();
        let _ = v;
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
}
