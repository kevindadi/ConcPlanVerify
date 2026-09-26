use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx, rx) = mpsc::sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let s_handle = thread::spawn(move || {
        // s: mutex_lock main::m; mutex_unlock main::m; channel_send main::ch1
        {
            let _guard = m_s.lock().unwrap();
        }
        tx.send(1).unwrap();
    });

    let m_r = Arc::clone(&m);
    let r_handle = thread::spawn(move || {
        // r: mutex_lock main::m; mutex_unlock main::m; channel_recv main::ch1
        {
            let _guard = m_r.lock().unwrap();
        }
        let _val = rx.recv().unwrap();
    });

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
}
