mod cir_trace;
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() { cir_trace::init();
    // R2: Unbuffered channel (capacity 0) ensures send and recv must meet.
    let (tx, rx) = sync_channel::<i32>(0);

    // R1: Start sender task s1
    let s1_handle = cir_trace::spawn("s1_handle", move || {
        // s1: channel_send main::ch value 1
        cir_trace::record("channel_send", "tx"); tx.send(1).expect("send failed");
    });

    // R1: Start receiver task r
    let r_handle = cir_trace::spawn("r_handle", move || {
        // r: channel_recv main::ch into val
        cir_trace::record("channel_recv", "rx"); let _val = rx.recv().expect("recv failed");
    });

    // Join both tasks to ensure termination (R4, R5)
    s1_handle.join().expect("s1 panicked");
    r_handle.join().expect("r panicked");

    // R7: Print exactly the required line
    println!("DONE done=1");
 cir_trace::finish();}
