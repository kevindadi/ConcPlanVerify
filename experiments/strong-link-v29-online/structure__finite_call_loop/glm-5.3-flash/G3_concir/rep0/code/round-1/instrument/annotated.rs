mod cir_trace;
// ConcIR design: sequential_calls
// main::main calls main::helper twice, then returns.

fn helper() {
    // main::helper body: return {}
}

fn main() { crate::cir_trace::init();
    // main::main body:
    // s1: call main::helper
    helper();
    // s2: call main::helper
    helper();
    // s3: return
    println!("DONE done=1");
 crate::cir_trace::finish();}
