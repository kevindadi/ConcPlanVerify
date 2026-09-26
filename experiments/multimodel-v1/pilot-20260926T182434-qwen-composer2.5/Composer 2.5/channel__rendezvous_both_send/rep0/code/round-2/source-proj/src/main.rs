use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::thread;

type Ch = (
    std::sync::mpsc::Sender<i32>,
    std::sync::mpsc::Receiver<i32>,
);

fn s1(ch: Arc<Ch>) {
    ch.0.send(1).unwrap();
}

fn r(ch: Arc<Ch>) {
    let mut v: i32 = 0;
    v = ch.1.recv().unwrap();
    let _ = v;
}

fn main() {
    let ch = Arc::new(sync_channel::<i32>(0));
    let ch_r = Arc::clone(&ch);

    let h_s1 = thread::spawn(move || s1(ch));
    let h_r = thread::spawn(move || r(ch_r));

    h_s1.join().unwrap();
    h_r.join().unwrap();

    println!("DONE done=1");
}
