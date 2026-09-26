mod cir_trace;
use std::sync::mpsc;
use std::thread;

fn s1(ch: mpsc::SyncSender<i32>) {
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
}

fn r(ch: mpsc::Receiver<i32>) {
    cir_trace::record("channel_recv", "ch"); let v = ch.recv().unwrap();
    let _ = v;
}

fn main() { cir_trace::init();
    let ch = mpsc::sync_channel::<i32>(0);
    let tx = ch.0;
    let rx = ch.1;

    thread::scope(|scope| {
        scope.spawn(|| s1(tx));
        scope.spawn(|| r(rx));
    });

    println!("DONE done=1");
 cir_trace::finish();}
