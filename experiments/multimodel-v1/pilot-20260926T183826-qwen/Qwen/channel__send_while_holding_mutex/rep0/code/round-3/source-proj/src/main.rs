use std::sync::{Arc, Mutex, mpsc};
use std::thread;

fn ch1_send(ch1: &(mpsc::SyncSender<i32>, mpsc::Receiver<i32>), v: i32) {
    let (tx, _rx) = ch1;
    tx.send(v).unwrap();
}

fn ch1_recv(ch1: &(mpsc::SyncSender<i32>, mpsc::Receiver<i32>)) -> i32 {
    let (_tx, rx) = ch1;
    rx.recv().unwrap()
}

fn ch2_send(ch2: &(mpsc::SyncSender<i32>, mpsc::Receiver<i32>), v: i32) {
    let (tx, _rx) = ch2;
    tx.send(v).unwrap();
}

fn ch2_recv(ch2: &(mpsc::SyncSender<i32>, mpsc::Receiver<i32>)) -> i32 {
    let (_tx, rx) = ch2;
    rx.recv().unwrap()
}

fn main() {
    let m = Arc::new(Mutex::new(()));

    // ch1: capacity 0 (rendezvous)
    let ch1 = Arc::new(mpsc::sync_channel::<i32>(0));
    // ch2: capacity 0 (rendezvous)
    let ch2 = Arc::new(mpsc::sync_channel::<i32>(0));

    let m_s = Arc::clone(&m);
    let ch1_s = Arc::clone(&ch1);
    let ch2_s = Arc::clone(&ch2);
    let s_handle = thread::spawn(move || {
        // s: mutex_lock main::m; mutex_unlock main::m
        {
            let _guard = m_s.lock().unwrap();
        }
        // call main::ch1_send with arg 1
        ch1_send(&*ch1_s, 1);
        // call main::ch2_recv
        let _ack = ch2_recv(&*ch2_s);
    });

    let m_r = Arc::clone(&m);
    let ch1_r = Arc::clone(&ch1);
    let ch2_r = Arc::clone(&ch2);
    let r_handle = thread::spawn(move || {
        // call main::ch1_recv
        let _val = ch1_recv(&*ch1_r);
        // r: mutex_lock main::m; mutex_unlock main::m
        {
            let _guard = m_r.lock().unwrap();
        }
        // call main::ch2_send with arg 1
        ch2_send(&*ch2_r, 1);
    });

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
}
