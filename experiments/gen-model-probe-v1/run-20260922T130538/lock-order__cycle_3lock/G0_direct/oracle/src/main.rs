mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

/// Each worker holds its two locks simultaneously while doing critical work.
/// Locks are always acquired in ascending global index order, which breaks
/// the circular wait and guarantees every schedule terminates.
fn worker(id: usize, first: Arc<Mutex<()>>, second: Arc<Mutex<()>>) {
    let guard1 = first.lock().unwrap();
    let guard2 = second.lock().unwrap();

    // Critical section: both locks are held here.
    let mut acc = 0u64;
    for i in 0..10_000u64 {
        acc = acc.wrapping_mul(31).wrapping_add(i ^ (id as u64));
    }
    std::hint::black_box(acc);

    // Releasing both locks before finishing (explicit drops for clarity).
    drop(guard2);
    drop(guard1);
}

fn main() { cir_trace::init();
    // Three shared locks.
    let locks = [
        Arc::new(Mutex::new_named("locks_mutex0", ())),
        Arc::new(Mutex::new_named("locks_mutex1", ())),
        Arc::new(Mutex::new_named("locks_mutex2", ())),
    ];

    // Worker 0 needs locks {0,1}, worker 1 needs {1,2}, worker 2 needs {2,0}.
    let mut handles = Vec::new();
    for i in 0..3usize {
        let a = i;
        let b = (i + 1) % 3;
        // Impose a global acquisition order (lower index first) so that no
        // cycle of workers each holding one lock and awaiting another can form.
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        let first = Arc::clone(&locks[lo]);
        let second = Arc::clone(&locks[hi]);
        handles.push(thread::spawn(move || worker(i, first, second)));
    }

    // Main finishes only after all workers have finished.
    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
