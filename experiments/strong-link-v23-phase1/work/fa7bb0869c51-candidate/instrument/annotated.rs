mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Channel that can hold at most one value.
// Uses a mutex and condvar for synchronization.
struct Channel {
    lock: Mutex<Option<i32>>,
    not_full: Condvar,
    not_empty: Condvar,
}

impl Channel {
    fn new() -> Channel {
        Channel {
            lock: Mutex::new(None),
            not_full: Condvar::new(),
            not_empty: Condvar::new(),
        }
    }

    // Send a value into the channel. Blocks if the channel is full.
    fn send(&self, value: i32) {
        let mut guard = self.lock.lock().unwrap();
        while guard.is_some() {
            // Wait until the channel is not full (i.e., empty).
            guard = self.not_empty.wait(guard).unwrap();
        }
        *guard = Some(value);
        // Notify that the channel is no longer empty.
        self.not_full.notify_one();
    }

    // Receive a value from the channel. Blocks if the channel is empty.
    fn recv(&self) -> i32 {
        let mut guard = self.lock.lock().unwrap();
        while guard.is_none() {
            // Wait until the channel is not empty.
            guard = self.not_full.wait(guard).unwrap();
        }
        let value = guard.take().unwrap();
        // Notify that the channel is no longer full.
        self.not_empty.notify_one();
        value
    }
}

fn main() { cir_trace::init();
    // Shared resources: m (lock) and ch (channel).
    // The problem states both roles occasionally need one shared lock 'm'.
    // We create an Arc<Mutex<...>> for the shared lock m.
    // Note: R5 says "No role may wait on the channel while holding the shared lock that the other role needs."
    // This means we must be careful with the order of acquiring locks.
    // The channel itself has its own internal mutex. The shared lock 'm' is separate.
    
    let m = Arc::new(Mutex::new_named("m_mutex0#1842", 0));
    let ch = Arc::new(Channel::new());

    // Sender role
    let sender_m = Arc::clone(&m);
    let sender_ch = Arc::clone(&ch);
    let sender_handle = cir_trace::spawn("sender_handle#2006", move || {
        // Sender passes two values in order.
        
        // First value
        // Occasionally need shared lock m. Let's say we acquire it before sending first value.
        // But R5: No role may wait on the channel while holding the shared lock.
        // So if we hold m, we cannot block on ch.send(). 
        // Strategy: Acquire m, do something quick, release m, then send.
        // Or: Send first (may block), then acquire m? But if we block on send while holding m, that violates R5.
        // So we must NOT hold m when calling ch.send().
        
        // Let's structure it as:
        // 1. Acquire m briefly (to satisfy "occasionally need one shared lock")
        // 2. Release m
        // 3. Send first value via ch (may block, but not holding m)
        
        {
            let _guard = sender_m.lock().unwrap();
            // Do some work with m
        }
        
        cir_trace::record("channel_send", "sender_ch"); sender_ch.send(1);
        
        // Second value
        // Again, acquire m briefly, release, then send.
        {
            let _guard = sender_m.lock().unwrap();
            // Do some work with m
        }
        
        cir_trace::record("channel_send", "sender_ch"); sender_ch.send(2);
    });

    // Receiver role
    let receiver_m = Arc::clone(&m);
    let receiver_ch = Arc::clone(&ch);
    let receiver_handle = cir_trace::spawn("receiver_handle#3322", move || {
        // Receiver takes two values.
        
        // First value
        // Acquire m briefly, release, then recv.
        {
            let _guard = receiver_m.lock().unwrap();
            // Do some work with m
        }
        
        cir_trace::record("channel_recv", "receiver_ch"); let v1 = receiver_ch.recv();
        assert_eq!(v1, 1);
        
        // Second value
        {
            let _guard = receiver_m.lock().unwrap();
            // Do some work with m
        }
        
        cir_trace::record("channel_recv", "receiver_ch"); let v2 = receiver_ch.recv();
        assert_eq!(v2, 2);
    });

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
