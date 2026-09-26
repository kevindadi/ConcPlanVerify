fn helper() {
    // Helper does nothing as per design
}

fn main() {
    // Main calls helper twice sequentially as per design
    helper();
    helper();
    
    // Print the required terminal line
    println!("DONE done=1");
}
