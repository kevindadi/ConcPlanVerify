mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn sender(ch: SyncSender<i32>, m: Arc<Mutex<()>>) {
    // Occasionally need the shared lock, but never while waiting on ch.
    {
        let _guard = m.lock().unwrap();
        // shared-lock critical section
    }

    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();

    {
        let _guard = m.lock().unwrap();
        // shared-lock critical section
    }

    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(ch: Receiver<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
        // shared-lock critical section
    }

    cir_trace::record("channel_recv", "ch"); let first = ch.recv().unwrap();
    assert_eq!(first, 1);

    {
        let _guard = m.lock().unwrap();
        // shared-lock critical section
    }

    cir_trace::record("channel_recv", "ch"); let second = ch.recv().unwrap();
    assert_eq!(second, 2);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#879", ()));
    let (ch, rx) = sync_channel(1);

    let sender_m = Arc::clone(&m);
    let sender_handle = cir_trace::spawn("sender#985", move || sender(ch, sender_m));

    let receiver_m = Arc::clone(&m);
    let receiver_handle = cir_trace::spawn("receiver#1094", move || receiver(rx, receiver_m));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
