mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{channel, Receiver, Sender};

use std::thread;

static lk: Mutex<()> = Mutex::new_named("lk_mutex0", ());

fn send(tx: &Sender<i32>, v: i32) {
    cir_trace::record("channel_send", "tx"); let _ = tx.send(v);
}

fn recv(rx: &Receiver<i32>) -> i32 {
    cir_trace::record("channel_recv", "rx"); rx.recv().unwrap()
}

fn s(tx1: Sender<i32>, rx2: Receiver<i32>) {
    {
        let _g = lk.lock().unwrap();
    }
    send(&tx1, 1);
    let _ = recv(&rx2);
}

fn r(rx1: Receiver<i32>, tx2: Sender<i32>) {
    {
        let _g = lk.lock().unwrap();
    }
    let _ = recv(&rx1);
    send(&tx2, 1);
}

fn main() { cir_trace::init();
    let (tx1, rx1) = channel::<i32>();
    let (tx2, rx2) = channel::<i32>();

    let hs = cir_trace::spawn("s", move || s(tx1, rx2));
    let hr = cir_trace::spawn("r", move || r(rx1, tx2));

    hs.join().unwrap();
    hr.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
