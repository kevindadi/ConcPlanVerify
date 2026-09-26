mod cir_trace;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx): (Sender<i32>, Receiver<i32>) = channel();

    let s1_handle = thread::Builder::new().name("main::s1".to_string()).spawn(move || {
        cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
    }).unwrap();

    let r_handle = thread::Builder::new().name("main::r".to_string()).spawn(move || {
        cir_trace::record("channel_recv", "rx"); let _val = rx.recv().unwrap();
    }).unwrap();

    s1_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
