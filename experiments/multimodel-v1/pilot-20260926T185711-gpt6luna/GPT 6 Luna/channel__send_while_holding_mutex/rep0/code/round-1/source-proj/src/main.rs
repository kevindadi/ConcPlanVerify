use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn s(m: Arc<Mutex<i32>>, ch1: SyncSender<i32>, ch2: Receiver<i32>) {
    {
        let mut n = m.lock().unwrap();
        *n = 1;
    }
    ch1.send(1).unwrap();
    let reply = ch2.recv().unwrap();
    let _ = reply;
}

fn r(m: Arc<Mutex<i32>>, ch1: Receiver<i32>, ch2: SyncSender<i32>) {
    let received = ch1.recv().unwrap();
    {
        let mut n = m.lock().unwrap();
        *n = 2;
    }
    ch2.send(received).unwrap();
}

fn main() {
    let m = Arc::new(Mutex::new(0));
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let s_handle = {
        let m = Arc::clone(&m);
        thread::spawn(move || s(m, ch1_tx, ch2_rx))
    };
    let r_handle = thread::spawn(move || r(m, ch1_rx, ch2_tx));

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
}
