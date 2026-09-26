use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::Mutex;
use std::thread;

fn chan_send(tx: &Sender<i32>, x: i32) {
    tx.send(x).unwrap();
}

fn chan_recv(rx: &Receiver<i32>) -> i32 {
    rx.recv().unwrap()
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

fn main() {
    let lk = Mutex::new(());
    let (tx1, rx1) = channel::<i32>();
    let (tx2, rx2) = channel::<i32>();

    let lk_ref = &lk;
    let tx1_ref = &tx1;
    let rx1_ref = &rx1;
    let tx2_ref = &tx2;
    let rx2_ref = &rx2;

    let hs = thread::spawn(move || {
        s(lk_ref, tx1_ref);
        let _ = tx2_ref.send(1);
    });

    let hr = thread::spawn(move || {
        r(lk_ref, rx1_ref);
        let _ = rx2_ref.recv();
    });

    hs.join().unwrap();
    hr.join().unwrap();

    println!("DONE done=1");
}
