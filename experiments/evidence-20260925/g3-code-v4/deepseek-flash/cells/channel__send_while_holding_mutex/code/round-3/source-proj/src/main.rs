use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (ch1_tx, ch1_rx) = mpsc::sync_channel::<i32>(0);
    let (_ch2_tx, _ch2_rx) = mpsc::sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let s = thread::spawn(move || {
        {
            let _g = m_s.lock().unwrap();
        }
        ch1_tx.send(1).unwrap();
        {
            let _g = m_s.lock().unwrap();
        }
    });

    let m_r = Arc::clone(&m);
    let r = thread::spawn(move || {
        let mut v: i32 = 0;
        {
            let _g = m_r.lock().unwrap();
        }
        v = ch1_rx.recv().unwrap();
        {
            let _g = m_r.lock().unwrap();
        }
        let _ = v;
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
}
