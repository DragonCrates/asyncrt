#![no_std]
#![no_main]
use dstd::prelude::*;
use dstd::thread::usleep;

asyncrt::main!(main);
async fn main() {
    let mut tasks = vec![];
    for i in 1..=10 {
        tasks.push(asyncrt::spawn(async move {
            println!("Begin blocking spawn {i}...");
            asyncrt::spawn_blocking(|| {
                usleep(1000);
            }).await;
            println!("Thread {i} finished");
        }));
    }

    for task in tasks {
        task.await;
    }

    println!("As you can see, all tasks completed simultaneously, even though there is only one thread driving the runtime");
}
