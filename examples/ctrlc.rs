#![no_std]
#![no_main]
use dstd::prelude::*;

asyncrt::main!(main);
async fn main() {
    println!("Press Ctrl+C to exit");
    asyncrt::signal::ctrl_c().await;
    println!("\nExiting...");
}
