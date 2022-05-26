#[allow(unused_must_use, dead_code)]

use serde::Deserialize;
use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Sender, Receiver};
use anyhow::{Result, Error, anyhow};
use std::time::{Duration, Instant};
use std::env;
use std::fs::File;

mod config;

fn main() {
    println!("Hello, world!");

    let args: Vec<String> = env::args().collect();
    let config = config::get_config(&args);

    let mut rdr = csv::ReaderBuilder::new().has_headers(false).from_reader(File::open(&config.input_path).unwrap());

    let (tx, mut rx) = mpsc::channel(5);

    let runtime = Builder::new_multi_thread()
        .worker_threads(config.worker_amt)
        .thread_name("my-custom-name")
        .thread_stack_size(3 * 1024 * 1024)
        .enable_time()
        .enable_io()
        .build()
        .unwrap();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build().unwrap();

    let start = Instant::now();

    let mut handles = Vec::with_capacity(20);
    for result in rdr.deserialize() {
        let csv_input: CsvInput = result.unwrap();
        let handle = runtime.spawn(request_dopa(client.clone(), csv_input, tx.clone()));
        handles.push(handle);
    }
    drop(tx);

    let listen_handler = runtime.spawn(async move {
        while let Some(res) = rx.recv().await {
            println!("Got {:?}", res);
        }
    });

    for handle in handles {
        runtime.block_on(handle);
    }
    runtime.block_on(listen_handler);
    println!("end elapse time {:?}", start.elapsed());
}

async fn request_dopa(client: reqwest::Client, csv_input: CsvInput, tx: Sender<Result<Response, ResponseError>>) -> Result<()> {
    let start = Instant::now();

    println!("request borrower id {}", csv_input.cid);

    let result = client.post(format!("http://ddeext-vip.pro.dsl:8086/CheckStatus/POPStatusService.asmx"))
        .header("Content-Type", "text/xml; charset=utf-8")
        .header("SOAPAction", "http://tempuri.org/CheckDeathStatus")
        .body(
            format!(
                r#"
                <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:api="http://tempuri.org/CheckDeathStatus"><soapenv:Header/><soapenv:Body><CheckDeathStatus xmlns="http://tempuri.org/"><pid>{}</pid><dob>{}</dob></CheckDeathStatus></soapenv:Body></soapenv:Envelope>
                "#, 
                csv_input.cid, csv_input.dob
            )
        )
        .send().await;
    let duration = start.elapsed();
    println!("borrower id {} elapse time {:?}", csv_input.cid, duration);

    let result1: Result<Response, ResponseError> = match result {
        
        Ok(res) => {
            let status_code = res.status().as_str().to_string();
            let body = res.text_with_charset("UTF-8").await.unwrap_or(String::from(""));
            let mut new_body = String::from("");
            if status_code == "200" {
                if let (Some(idx_start), Some(idx_stop)) = (body.find("<stCode>"), body.find("</stCode>")) {
                    let idx_start = idx_start + 8;
                    new_body.push_str(&body[idx_start..idx_stop]);
                }
                if let (Some(idx_start), Some(idx_stop)) = (body.find("<stDesc>"), body.find("</stDesc>")) {
                    let idx_start = idx_start + 8;
                    new_body.push_str(&body[idx_start..idx_stop]);
                }
            }
            Ok(Response {
                request_id: csv_input.cid.clone(),
                status_code: status_code,
                body: body,
                elapse: duration
            })
        },
        Err(error) => {
            Err(ResponseError {
                request_id: csv_input.cid.clone(),
                message: error.to_string(),
                elapse: duration
            })
        }
    };
    
    tx.send(result1).await;
    Ok(())
}

async fn request(client: reqwest::Client, csv_input: CsvInput, tx: Sender<Result<Response, ResponseError>>) -> Result<()> {
    let start = Instant::now();

    println!("request borrower id {}", csv_input.cid);
    // let result = client.get(format!("http://localhost:8088/dms/api/v1/borrower/{}", id)).send().await;
    let result = client.get(format!("http://localhost:8088/demo/api/v1/timeout/9900")).send().await;
    let duration = start.elapsed();
    println!("borrower id {} elapse time {:?}", csv_input.cid, duration);

    let result1: Result<Response, ResponseError> = match result {
        Ok(res) => {
            Ok(Response {
                request_id: csv_input.cid.clone(),
                status_code: res.status().as_str().to_string(),
                body: res.text().await.unwrap_or(String::from("")),
                elapse: duration
            })
        },
        Err(error) => {
            Err(ResponseError {
                request_id: csv_input.cid.clone(),
                message: error.to_string(),
                elapse: duration
            })
        }
    };
    
    tx.send(result1).await;
    Ok(())
}



#[derive(Debug, Deserialize)]
struct CsvInput {
    cid: String,
    dob: String
}

#[derive(Debug)]
struct Response {
    request_id: String,
    status_code: String,
    body: String,
    elapse: Duration
}

#[derive(Debug)]
struct ResponseError {
    request_id: String,
    message: String,
    elapse: Duration
}
