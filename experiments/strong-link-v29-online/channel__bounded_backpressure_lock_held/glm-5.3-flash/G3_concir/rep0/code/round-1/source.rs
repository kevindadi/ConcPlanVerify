use std::sync::{Arc, Mutex};
use std::sync::mpsc::sync_channel;
use std::thread;

fn sender(ch_tx: std::sync::mpsc::SyncSender<i32>, m: Arc<Mutex<bool>>) {
    // channel_send { channel: main::ch, value: 1 }
    ch_tx.send(1).unwrap();
    // channel_send { channel: main::ch, value: 2 }
    // (blocks while the one-slot channel is full, i.e. until the
    //  receiver has taken the first value)
    ch_tx.send(2).unwrap();
    // mutex_lock { resource: main::m }
    let guard = m.lock().unwrap();
    // read_shared { resource: main::done }
    let _ = *guard;
    // mutex_unlock { resource: main::m }
    drop(guard);
}

fn receiver(ch_rx: std::sync::mpsc::Receiver<i32>, m: Arc<Mutex<bool>>) {
    // channel_recv { channel: main::ch, dst: _ }
    let _ = ch_rx.recv().unwrap();
    // channel_recv { channel: main::ch, dst: _ }
    // (blocks while the channel is empty)
    let _ = ch_rx.recv().unwrap();
    // mutex_lock { resource: main::m }
    let mut guard = m.lock().unwrap();
    // write_shared { resource: main::done, expr: true }
    *guard = true;
    // mutex_unlock { resource: main::m }
    drop(guard);
}

fn main() {
    // Resource: done (var, protected by m) stored inside the mutex m.
    let done: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    // Resource: ch (channel, capacity 1).
    let (ch_tx, ch_rx) = sync_channel::<i32>(1);

    let m_sender = Arc::clone(&done);
    let t_sender = thread::spawn(move || sender(ch_tx, m_sender));

    let m_receiver = Arc::clone(&done);
    let t_receiver = thread::spawn(move || receiver(ch_rx, m_receiver));

    t_sender.join().unwrap();
    t_receiver.join().unwrap();

    // Read shared state only to print the required terminal line.
    let guard = done.lock().unwrap();
    let flag = if *guard { 1 } else { 0 };
    drop(guard);
    println!("DONE done={}", flag);
}
