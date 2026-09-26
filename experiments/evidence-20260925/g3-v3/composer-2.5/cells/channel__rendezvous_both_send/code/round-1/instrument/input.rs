use std::sync::mpsc;
use std::thread;

fn s1(ch: mpsc::SyncSender<i32>) {
    ch.send(1).unwrap();
}

fn r(ch: mpsc::Receiver<i32>) {
    let v = ch.recv().unwrap();
    let _ = v;
}

fn main() {
    let ch = mpsc::sync_channel::<i32>(0);
    let tx = ch.0;
    let rx = ch.1;

    thread::scope(|scope| {
        scope.spawn(|| s1(tx));
        scope.spawn(|| r(rx));
    });

    println!("DONE done=1");
}
