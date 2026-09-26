mod cir_trace;
use concir_sync::Semaphore;
use std::sync::{mpsc::sync_channel, Arc};
use std::thread;

fn s(
    ch1: std::sync::mpsc::SyncSender<i32>,
    ch2: std::sync::mpsc::Receiver<i32>,
    lock: Arc<Semaphore>,
) {
    {
        let _permit = lock.acquire();
    }
    cir_trace::record("channel_send", "ch1"); ch1.send(1).unwrap();
    cir_trace::record("channel_recv", "ch2"); let _ = ch2.recv().unwrap();
}

fn r(
    ch1: std::sync::mpsc::Receiver<i32>,
    ch2: std::sync::mpsc::SyncSender<i32>,
    lock: Arc<Semaphore>,
) {
    cir_trace::record("channel_recv", "ch1"); let _ = ch1.recv().unwrap();
    {
        let _permit = lock.acquire();
    }
    cir_trace::record("channel_send", "ch2"); ch2.send(0).unwrap();
}

fn main() { cir_trace::init();
    let lock = Semaphore::new_named("lock_semaphore0", 1);
    let ch1 = sync_channel::<i32>(0);
    let ch2 = sync_channel::<i32>(0);

    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    let ch1_tx = ch1.0;
    let ch1_rx = ch1.1;
    let ch2_tx = ch2.0;
    let ch2_rx = ch2.1;

    let ts = cir_trace::spawn("s", move || s(ch1_tx, ch2_rx, lock_s));
    let tr = cir_trace::spawn("r", move || r(ch1_rx, ch2_tx, lock_r));

    ts.join().unwrap();
    tr.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
