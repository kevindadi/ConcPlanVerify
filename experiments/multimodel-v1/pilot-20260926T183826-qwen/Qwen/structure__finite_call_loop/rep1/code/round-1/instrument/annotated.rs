mod cir_trace;
fn helper() {
    // Helper does nothing as per design
}

fn main() { cir_trace::init();
    // Main calls helper twice sequentially as per design
    helper();
    helper();
    
    // Print the required terminal line
    println!("DONE done=1");
 cir_trace::finish();}
