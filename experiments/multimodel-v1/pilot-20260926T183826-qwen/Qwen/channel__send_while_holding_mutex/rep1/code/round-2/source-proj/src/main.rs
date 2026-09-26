use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx1, rx1): (Sender<i32>, Receiver<i32>) = channel();
    let (tx2, rx2): (Sender<i32>, Receiver<i32>) = channel();

    // s: mutex_lock main::m; mutex_unlock main::m; channel_send main::ch1; channel_recv main::ch2
    let m_s = Arc::clone(&m);
    let tx1_s = tx1;
    let rx2_s = rx2;

    let s_handle = thread::spawn(move || {
        {
            let _guard = m_s.lock().unwrap();
        }
        tx1_s.send(1).unwrap();
        let _tmp_s: i32 = rx2_s.recv().unwrap();
    });

    // r: channel_recv main::ch1; mutex_lock main::m; mutex_unlock main::m; channel_send main::ch2
    let m_r = Arc::clone(&m);
    let rx1_r = rx1;
    let tx2_r = tx2;

    let r_handle = thread::spawn(move || {
        let _tmp_r: i32 = rx1_r.recv().unwrap();
        {
            let _guard = m_r.lock().unwrap();
        }
        tx2_r.send(1).unwrap();
    });

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
}
