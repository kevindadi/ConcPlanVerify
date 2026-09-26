use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let t_s = thread::spawn(move || s(m_s, ch1_tx, ch2_rx));

    let m_r = Arc::clone(&m);
    let t_r = thread::spawn(move || r(m_r, ch1_rx, ch2_tx));

    let ack = t_s.join().unwrap();
    let _v = t_r.join().unwrap();
    println!("DONE done={ack}");
}

fn s(m: Arc<Mutex<()>>, ch1: SyncSender<i32>, ch2: Receiver<i32>) -> i32 {
    let mut ack = 0;
    let guard = m.lock().unwrap();
    drop(guard);
    ch1.send(1).unwrap();
    ack = ch2.recv().unwrap();
    ack
}

fn r(m: Arc<Mutex<()>>, ch1: Receiver<i32>, ch2: SyncSender<i32>) -> i32 {
    let mut v = 0;
    let guard = m.lock().unwrap();
    drop(guard);
    v = ch1.recv().unwrap();
    ch2.send(1).unwrap();
    v
}
