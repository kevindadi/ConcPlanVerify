use std::sync::mpsc::sync_channel;
use std::thread;

fn s1(ch: std::sync::mpsc::SyncSender<i32>) {
    ch.send(1).unwrap();
}

fn r(ch: std::sync::mpsc::SyncReceiver<i32>) {
    let mut v = 0;
    v = ch.recv().unwrap();
    let _ = v;
}

fn main() {
    let (tx, rx) = sync_channel::<i32>(0);

    let h_s1 = thread::spawn(move || s1(tx));
    let h_r = thread::spawn(move || r(rx));

    h_s1.join().unwrap();
    h_r.join().unwrap();

    println!("DONE done=1");
}
