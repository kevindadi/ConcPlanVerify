mod cir_trace;
// ConcIR design: main_calls_helper_twice
// module main:
//   main:   call main::helper; call main::helper; return
//   helper: return
//
// No resources (mutexes, condvars, channels, semaphores, atomics) are
// declared, so there is no shared state and no contention. The two
// auxiliary calls are sequenced by the calling thread itself, so each
// call runs to completion before the next one starts, and every
// schedule terminates.

fn helper() {
    // s1: return
}

fn main() { cir_trace::init();
    // s1: call main::helper
    helper();

    // s2: call main::helper
    helper();

    // s3: return -> terminal line required by R5
    println!("DONE done=1");
 cir_trace::finish();}
