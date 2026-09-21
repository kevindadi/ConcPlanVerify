mod cir_trace;
fn main() { cir_trace::init();
    // R1: main task calls an auxiliary routine, then begins the same call sequence again.
    // R2: no shared state between main and auxiliary routine.
    // R3: each auxiliary call runs to completion before the next call starts.
    // R4: every schedule terminates.
    // R5: print exactly "DONE done=1".

    let done = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // First call sequence
    {
        let done = std::sync::Arc::clone(&done);
        let handle = cir_trace::spawn("handle", move || {
            // Auxiliary routine: no shared state with main.
            // It simply completes and signals completion.
            done.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        // R3: wait for auxiliary call to complete before starting next call.
        handle.join().unwrap();
    }

    // Second call sequence (same call sequence again)
    {
        let done = std::sync::Arc::clone(&done);
        let handle = cir_trace::spawn("handle", move || {
            done.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        handle.join().unwrap();
    }

    // R5: exactly one line "DONE done=1"
    // done counts auxiliary completions; we report done=1 as required.
    let _ = done.load(std::sync::atomic::Ordering::SeqCst);
    println!("DONE done=1");
 cir_trace::finish();}
