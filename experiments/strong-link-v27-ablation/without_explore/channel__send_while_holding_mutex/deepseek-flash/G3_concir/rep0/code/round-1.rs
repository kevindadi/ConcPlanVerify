use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(0));

    let (ch1_tx, ch1_rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);
    let (ch2_tx, ch2_rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let m_s = Arc::clone(&m);
    let s_handle = thread::spawn(move || {
        {
            let mut n = m_s.lock().unwrap();
            *n = 1;
        }

        let msg = 1;
        ch1_tx.send(msg).unwrap();

        let ack = ch2_rx.recv().unwrap();

        {
            let mut n = m_s.lock().unwrap();
            *n = ack;
        }
    });

    let m_r = Arc::clone(&m);
    let r_handle = thread::spawn(move || {
        {
            let mut n = m_r.lock().unwrap();
            *n = 1;
        }

        let val = ch1_rx.recv().unwrap();
        let ack = val;
        ch2_tx.send(ack).unwrap();

        {
            let mut n = m_r.lock().unwrap();
            *n = val;
        }
    });

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    let done = 1;
    println!("DONE done={}", done);
}
