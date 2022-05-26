use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Sender, Receiver};
use tokio::task::JoinHandle;
use anyhow::{Result, Error};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");

    let (tx, mut rx) = mpsc::channel(5);

    let start = Instant::now();

    // let mut handles: Vec<JoinHandle<Result<String, _>>> = Vec::with_capacity(20);
    for i in 1..11 {
        // let handle = tokio::spawn(request(i, tx.clone()));
        // handles.push(handle);

        let txn = tx.clone();
        tokio::spawn(
            request(i, txn)
        );
    }
    tokio::spawn(
        request(10, tx)
    );


    while let Some(res) = rx.recv().await {
        println!("Got {}", res);
    }

    // for handle in handles {
    //     runtime.block_on(handle);
    // }
    println!("end elapse time {:?}", start.elapsed());

    Ok(())
}

async fn request(id: i32, tx: Sender<String>) -> Result<String, Error>{
    println!("request borrower id {}", id);
    let start = Instant::now();
    let res = reqwest::get(format!("http://localhost:8088/dms/api/v1/borrower/{}", id)).await?.text().await?;
    let duration = start.elapsed();
    println!("borrower id {} elapse time {:?}", id, duration);
    tx.send(res.clone()).await;
    Ok(res)
}