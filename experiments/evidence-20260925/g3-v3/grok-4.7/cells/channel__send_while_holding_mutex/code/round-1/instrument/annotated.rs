mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc, mpsc};

fn s(m: Arc<Mutex<()>>, ch1: mpsc::SyncSender<i32>, ch2: mpsc::Receiver<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch1"); ch1.send(1).unwrap();
    cir_trace::record("channel_recv", "ch2"); let _ack = ch2.recv().unwrap();
}

fn r(m: Arc<Mutex<()>>, ch1: mpsc::Receiver<i32>, ch2: mpsc::SyncSender<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "ch1"); let _v = ch1.recv().unwrap();
    cir_trace::record("channel_send", "ch2"); ch2.send(1).unwrap();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (ch1_tx, ch1_rx) = mpsc::sync_channel(0);
    let (ch2_tx, ch2_rx) = mpsc::sync_channel(0);

    let m_s = Arc::clone(&m);
    let m_r = Arc::clone(&m);

    std::thread::scope(|scope| {
        scope.spawn(move || s(m_s, ch1_tx, ch2_rx));
        scope.spawn(move || r(m_r, ch1_rx, ch2_tx));
    });

    println!("DONE done=1");
 cir_trace::finish();}
