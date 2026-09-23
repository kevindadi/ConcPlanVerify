mod cir_trace;
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx) = sync_channel::<i64>(0);

    let s1 = cir_trace::spawn("s1", move || {
        cir_trace::record("channel_send", "tx"); tx.send(1).expect("send failed");
    });

    let r = cir_trace::spawn("r", move || {
        cir_trace::record("channel_recv", "rx"); let got: i64 = rx.recv().expect("recv failed");
        let _ = got;
    });

    s1.join().expect("s1 panicked");
    r.join().expect("r panicked");

    let done: i64 = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
