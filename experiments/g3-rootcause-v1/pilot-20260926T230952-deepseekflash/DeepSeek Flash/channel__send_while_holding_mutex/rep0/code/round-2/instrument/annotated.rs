mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc};
use std::thread;

fn s(tx1: Sender<i32>, rx2: Receiver<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "tx1"); tx1.send(1).unwrap();
    cir_trace::record("channel_recv", "rx2"); let _ = rx2.recv().unwrap();
}

fn r(rx1: Receiver<i32>, tx2: Sender<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "rx1"); let _ = rx1.recv().unwrap();
    cir_trace::record("channel_send", "tx2"); tx2.send(1).unwrap();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (tx1, rx1) = channel::<i32>();
    let (tx2, rx2) = channel::<i32>();

    let m_s = Arc::clone(&m);
    let m_r = Arc::clone(&m);

    let hs = cir_trace::spawn("s", move || s(tx1, rx2, m_s));
    let hr = cir_trace::spawn("r", move || r(rx1, tx2, m_r));

    hs.join().unwrap();
    hr.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
