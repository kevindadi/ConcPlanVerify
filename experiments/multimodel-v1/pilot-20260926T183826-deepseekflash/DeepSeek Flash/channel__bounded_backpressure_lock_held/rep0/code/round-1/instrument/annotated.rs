mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn send(tx: &mpsc::SyncSender<i32>, v: i32) {
    cir_trace::record("channel_send", "tx"); tx.send(v).unwrap();
}

fn recv(rx: &mpsc::Receiver<i32>) -> i32 {
    cir_trace::record("channel_recv", "rx"); rx.recv().unwrap()
}

fn sender(m: Arc<Mutex<()>>, tx: mpsc::SyncSender<i32>) {
    {
        let _g = m.lock().unwrap();
    }
    send(&tx, 1);
    {
        let _g = m.lock().unwrap();
    }
    send(&tx, 2);
}

fn receiver(m: Arc<Mutex<()>>, rx: mpsc::Receiver<i32>) {
    {
        let _g = m.lock().unwrap();
    }
    let _v1 = recv(&rx);
    {
        let _g = m.lock().unwrap();
    }
    let _v2 = recv(&rx);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (tx, rx) = mpsc::sync_channel::<i32>(1);

    let m1 = Arc::clone(&m);
    let tx1 = tx.clone();
    let sender_handle = cir_trace::spawn("sender", move || {
        sender(m1, tx1);
    });

    let m2 = Arc::clone(&m);
    let receiver_handle = cir_trace::spawn("receiver", move || {
        receiver(m2, rx);
    });

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
