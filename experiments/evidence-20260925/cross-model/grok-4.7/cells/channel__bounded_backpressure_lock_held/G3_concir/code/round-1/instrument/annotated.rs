mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

struct Shared {
    done: i32,
}

fn sender(m: Arc<Mutex<Shared>>, tx: mpsc::SyncSender<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "tx"); tx.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<Shared>>, rx: mpsc::Receiver<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "rx"); let v1 = rx.recv().unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "rx"); let v2 = rx.recv().unwrap();
    let _ = (v1, v2);
}

fn println() {
    println!("DONE done=1");
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", Shared { done: 0 }));
    let ch = mpsc::sync_channel::<i32>(1);
    let (tx, rx) = ch;

    let m_sender = Arc::clone(&m);
    let m_receiver = Arc::clone(&m);

    let sender_thread = cir_trace::spawn("sender", move || {
        sender(m_sender, tx);
    });
    let receiver_thread = cir_trace::spawn("receiver", move || {
        receiver(m_receiver, rx);
    });

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    {
        let mut guard = m.lock().unwrap();
        guard.done = 1;
    }

    println();
 cir_trace::finish();}
