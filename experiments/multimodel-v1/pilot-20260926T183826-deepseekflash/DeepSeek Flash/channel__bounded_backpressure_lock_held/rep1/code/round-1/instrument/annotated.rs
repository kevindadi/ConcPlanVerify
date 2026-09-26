mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (tx, rx) = mpsc::sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let sender = cir_trace::spawn("sender", move || {
        sender(m_sender, tx);
    });

    let m_receiver = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver", move || {
        receiver(m_receiver, rx);
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn sender(m: Arc<Mutex<()>>, tx: mpsc::SyncSender<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    send(&tx, 1);
    {
        let _guard = m.lock().unwrap();
    }
    send(&tx, 2);
}

fn receiver(m: Arc<Mutex<()>>, rx: mpsc::Receiver<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    recv(&rx);
    {
        let _guard = m.lock().unwrap();
    }
    recv(&rx);
}

fn send(tx: &mpsc::SyncSender<i32>, v: i32) {
    cir_trace::record("channel_send", "tx"); tx.send(v).unwrap();
}

fn recv(rx: &mpsc::Receiver<i32>) {
    cir_trace::record("channel_recv", "rx"); let _v = rx.recv().unwrap();
}
