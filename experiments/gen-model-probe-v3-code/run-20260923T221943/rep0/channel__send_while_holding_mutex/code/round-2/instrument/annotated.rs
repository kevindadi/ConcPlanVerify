mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

use std::thread;

fn s(m: &Mutex<i64>, ch1: SyncSender<i64>, ch2_rx: Receiver<i64>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch1"); ch1.send(1).unwrap();
    cir_trace::record("channel_recv", "ch2_rx"); let ack = ch2_rx.recv().unwrap();
    let _ = ack;
}

fn r(m: &Mutex<i64>, ch1_rx: Receiver<i64>, ch2: SyncSender<i64>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "ch1_rx"); let v = ch1_rx.recv().unwrap();
    let _ = v;
    cir_trace::record("channel_send", "ch2"); ch2.send(1).unwrap();
}

fn main() { cir_trace::init();
    let m = Mutex::new_named("m_mutex0", 0i64);
    let (ch1, ch1_rx) = sync_channel::<i64>(0);
    let (ch2, ch2_rx) = sync_channel::<i64>(0);

    thread::scope(|scope| {
        let hs = scope.spawn(|| s(&m, ch1, ch2_rx));
        let hr = scope.spawn(|| r(&m, ch1_rx, ch2));
        hs.join().unwrap();
        hr.join().unwrap();
    });

    {
        let mut done = m.lock().unwrap();
        *done = 1;
    }

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
