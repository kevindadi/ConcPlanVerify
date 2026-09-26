mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc};
use std::thread;

fn chan_send(tx: &Sender<i32>, x: i32) {
    cir_trace::record("channel_send", "tx"); tx.send(x).unwrap();
}

fn chan_recv(rx: &Receiver<i32>) -> i32 {
    cir_trace::record("channel_recv", "rx"); rx.recv().unwrap()
}

fn s(lk: &Mutex<()>, tx: &Sender<i32>) {
    {
        let _guard = lk.lock().unwrap();
    }
    chan_send(tx, 1);
}

fn r(lk: &Mutex<()>, rx: &Receiver<i32>) {
    {
        let _guard = lk.lock().unwrap();
    }
    let _v = chan_recv(rx);
}

fn main() { cir_trace::init();
    let lk = Arc::new(Mutex::new_named("lk_mutex0", ()));
    let (tx1, rx1) = channel::<i32>();
    let (tx2, rx2) = channel::<i32>();

    let lk_s = Arc::clone(&lk);
    let hs = cir_trace::spawn("s", move || {
        s(&lk_s, &tx1);
        cir_trace::record("channel_send", "tx2"); let _ = tx2.send(1);
    });

    let lk_r = Arc::clone(&lk);
    let hr = cir_trace::spawn("r", move || {
        r(&lk_r, &rx1);
        cir_trace::record("channel_recv", "rx2"); let _ = rx2.recv();
    });

    hs.join().unwrap();
    hr.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
