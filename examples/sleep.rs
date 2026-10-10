#![no_std]
#![no_main]
use dstd::prelude::*;
use asyncrt::time::usleep;

asyncrt::main!(main);
async fn main() {
    println!("Sleep for 5 seconds...");
    for i in 1..=5 {
        usleep(1000).await;
        println!("{i}");
    }
}
