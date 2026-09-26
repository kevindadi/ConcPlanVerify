mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

struct Shared {
    count: i32,
    done: i32,
}

fn s(m: Arc<Mutex<Shared>>, ch1: SyncSender<i32>, ch2: Receiver<i32>) {
    {
        let mut guard = m.lock().unwrap();
        guard.count += 1;
    }

    cir_trace::record("channel_send", "ch1"); ch1.send(1).unwrap();
    cir_trace::record("channel_recv", "ch2"); let reply = ch2.recv().unwrap();

    {
        let mut guard = m.lock().unwrap();
        guard.count += 1;
    }

    let _ = reply;
}

fn r(m: Arc<Mutex<Shared>>, ch1: Receiver<i32>, ch2: SyncSender<i32>) {
    {
        let mut guard = m.lock().unwrap();
        guard.count += 1;
    }

    cir_trace::record("channel_recv", "ch1"); let message = ch1.recv().unwrap();
    cir_trace::record("channel_send", "ch2"); ch2.send(2).unwrap();

    {
        let mut guard = m.lock().unwrap();
        guard.count += 1;
    }

    let _ = message;
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", Shared { count: 0, done: 0 }));
    let (ch1_tx, ch1_rx) = sync_channel(0);
    let (ch2_tx, ch2_rx) = sync_channel(0);

    let s_m = Arc::clone(&m);
    let s_thread = cir_trace::spawn("s", move || s(s_m, ch1_tx, ch2_rx));

    let r_m = Arc::clone(&m);
    let r_thread = cir_trace::spawn("r", move || r(r_m, ch1_rx, ch2_tx));

    s_thread.join().unwrap();
    r_thread.join().unwrap();

    {
        let mut guard = m.lock().unwrap();
        guard.done = 1;
    }

    let guard = m.lock().unwrap();
    println!("DONE done={}", guard.done);
 cir_trace::finish();}
