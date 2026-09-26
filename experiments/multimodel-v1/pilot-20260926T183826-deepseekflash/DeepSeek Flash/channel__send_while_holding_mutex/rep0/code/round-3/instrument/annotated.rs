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

fn s(ch1_tx: Sender<i32>, ch2_rx: Receiver<i32>) {
    {
        let _g = lk.lock().unwrap();
    }
    send(&ch1_tx, 1);
    let _ = recv(&ch2_rx);
}

fn r(ch1_rx: Receiver<i32>, ch2_tx: Sender<i32>) {
    {
        let _g = lk.lock().unwrap();
    }
    let _ = recv(&ch1_rx);
    send(&ch2_tx, 1);
}

fn main() { cir_trace::init();
    let (ch1_tx, ch1_rx) = channel::<i32>();
    let (ch2_tx, ch2_rx) = channel::<i32>();

    let hs = cir_trace::spawn("s", move || s(ch1_tx, ch2_rx));
    let hr = cir_trace::spawn("r", move || r(ch1_rx, ch2_tx));

    hs.join().unwrap();
    hr.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
