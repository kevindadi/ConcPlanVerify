mod cir_trace;
use concir_sync::Semaphore;

fn helper(sem: &Semaphore) {
    let _permit = sem.acquire();
}

fn main() { cir_trace::init();
    let sem = Semaphore::new_named("sem_semaphore0", 1);
    
    // First call sequence
    helper(&sem);
    
    // Second call sequence (same as first, runs to completion before next starts per R3)
    helper(&sem);
    
    println!("DONE done=1");
 cir_trace::finish();}
