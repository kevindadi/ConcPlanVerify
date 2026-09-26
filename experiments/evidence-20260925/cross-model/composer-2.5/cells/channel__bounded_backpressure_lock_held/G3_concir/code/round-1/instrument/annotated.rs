mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc;
use std::sync::{Arc};
use std::thread;

struct SharedState {
    acc: i32,
    done: i32,
}

fn sender(m: Arc<Mutex<SharedState>>, ch: mpsc::SyncSender<i32>) {
    {
        let mut guard = m.lock().unwrap();
        guard.acc = guard.acc + 1;
    }
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
    {
        let mut guard = m.lock().unwrap();
        guard.acc = guard.acc + 1;
    }
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<SharedState>>, ch: mpsc::Receiver<i32>) {
    cir_trace::record("channel_recv", "ch"); let v1 = ch.recv().unwrap();
    let a1;
    {
        let guard = m.lock().unwrap();
        a1 = guard.acc;
    }
    cir_trace::record("channel_recv", "ch"); let v2 = ch.recv().unwrap();
    let a2;
    {
        let guard = m.lock().unwrap();
        a2 = guard.acc;
    }
    let _ = (v1, v2, a1, a2);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", SharedState { acc: 0, done: 0 }));
    let ch = mpsc::sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let ch_sender = ch.0.clone();
    let sender_handle = cir_trace::spawn("sender", move || sender(m_sender, ch_sender));

    let m_receiver = Arc::clone(&m);
    let ch_receiver = ch.1;
    let receiver_handle = cir_trace::spawn("receiver", move || receiver(m_receiver, ch_receiver));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    {
        let mut guard = m.lock().unwrap();
        guard.done = 1;
    }

    println!("DONE done=1");
 cir_trace::finish();}
