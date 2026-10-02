mod cir_trace;
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() { cir_trace::init();
    let ch = sync_channel::<()>(0);
    let (tx, rx) = ch;

    let s1 = cir_trace::spawn("s1#138", move || {
        cir_trace::record("channel_send", "tx"); tx.send(()).unwrap();
    });

    let r = cir_trace::spawn("r#213", move || {
        cir_trace::record("channel_recv", "rx"); rx.recv().unwrap();
    });

    s1.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
