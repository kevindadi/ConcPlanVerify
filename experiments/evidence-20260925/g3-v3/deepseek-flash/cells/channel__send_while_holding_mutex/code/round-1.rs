use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx1, rx1) = mpsc::sync_channel::<i32>(0);
    let (_tx2, _rx2) = mpsc::sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let tx1_s = tx1.clone();
    let s = thread::spawn(move || {
        {
            let _g = m_s.lock().unwrap();
        }
        tx1_s.send(1).unwrap();
        {
            let _g = m_s.lock().unwrap();
        }
    });

    let m_r = Arc::clone(&m);
    let r = thread::spawn(move || {
        {
            let _g = m_r.lock().unwrap();
        }
        let v: i32 = rx1.recv().unwrap();
        {
            let _g = m_r.lock().unwrap();
        }
        let _ = v;
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
}
