mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc, mpsc::{sync_channel, SyncSender, Receiver};use crate::cir_trace::sync::{Mutex};
use std::thread;

struct Shared {
    done: i32,
}

fn sender(tx: SyncSender<i32>, m: Arc<Mutex<Shared>>) {
    let mut i = 0;
    cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
    {
        let _guard = m.lock().unwrap();
        i += 1;
    }
    cir_trace::record("channel_send", "tx"); tx.send(2).unwrap();
    let _ = i;
}

fn receiver(rx: Receiver<i32>, m: Arc<Mutex<Shared>>) {
    let mut j = 0;
    {
        let _guard = m.lock().unwrap();
        j += 1;
    }
    cir_trace::record("channel_recv", "rx"); let x = rx.recv().unwrap();
    {
        let _guard = m.lock().unwrap();
        j += 1;
    }
    cir_trace::record("channel_recv", "rx"); let y = rx.recv().unwrap();
    let _ = (j, x, y);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#677", Shared { done: 0 }));
    let (tx, rx) = sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let m_receiver = Arc::clone(&m);

    let hs = cir_trace::spawn("sender#833", move || sender(tx, m_sender));
    let hr = cir_trace::spawn("receiver#891", move || receiver(rx, m_receiver));

    hs.join().unwrap();
    hr.join().unwrap();

    {
        let mut guard = m.lock().unwrap();
        guard.done = 1;
    }

    println!("DONE done=1");
 cir_trace::finish();}
