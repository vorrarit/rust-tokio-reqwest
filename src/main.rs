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
        while let Some(result) = rx.recv().await {
            print_result(&result);
        }
    });

    for handle in handles {
        runtime.block_on(handle);
    }
    runtime.block_on(listen_handler);
    println!("end elapse time {:?}", start.elapsed());
}

fn print_result(result: &Result<Response, ResponseError>) {
    match result {
        Ok(response) => {

            let status_code = response.status_code.clone();
            let body = response.body.clone();
            let mut result_string = String::from("success");
            let mut st_code = String::from("");
            let mut st_desc = String::from("");

            if status_code == "200" {
                if let Some(_idx_start) = body.find("The requested URL was rejected") {
                    result_string = "rejected".to_string();
                }
                // if there is errorDesc, there will not have stDesc
                if let (Some(idx_start), Some(idx_stop)) = (body.find("<errorDesc>"), body.find("</errorDesc>")) {
                    result_string = "error".to_string();
                    let idx_start = idx_start + 11;
                    if idx_start < idx_stop {
                        st_desc = body[idx_start..idx_stop].to_string();
                    }
                }
                if let (Some(idx_start), Some(idx_stop)) = (body.find("<stCode>"), body.find("</stCode>")) {
                    let idx_start = idx_start + 8;
                    if idx_start < idx_stop {
                        st_code = body[idx_start..idx_stop].to_string();
                    }
                }
                if st_desc == "" {
                    if let (Some(idx_start), Some(idx_stop)) = (body.find("<stDesc>"), body.find("</stDesc>")) {
                        let idx_start = idx_start + 8;
                        if idx_start < idx_stop {
                            st_desc = body[idx_start..idx_stop].to_string().clone();
                        }
                    }
                }
            }

            println!("{},{},{},{},{:?},{},{}", response.csv_input.cid, response.csv_input.dob, response.status_code, result_string, response.elapse, st_code, st_desc);
        },
        Err(error) => {
            println!("{},{},{},{},{:?},{},{}", error.csv_input.cid, error.csv_input.dob, "", "error", error.elapse, "", error.message);
        }
    }
}

fn format_dob(dob: &String) -> String {
    let v_dob: Vec<&str> = dob.split("-").collect();
    let mut i_dob = v_dob[0].parse::<i32>().unwrap();
    i_dob = i_dob + 543;
    let s_dob = format!("{}{}{}", i_dob, v_dob[1], v_dob[2]);
    return s_dob;
}

async fn request_dopa(client: reqwest::Client, csv_input: CsvInput, tx: Sender<Result<Response, ResponseError>>) -> Result<()> {
    let start = Instant::now();

    // println!("request borrower id {}", csv_input.cid);

    let url = format!("http://ddeext-vip.pro.dsl:8086/CheckStatus/POPStatusService.asmx");
    // let url = format!("http://localhost:8086/CheckStatus/POPStatusService.asmx");

    let s_dob = format_dob(&csv_input.dob);
    
    let result = client.post(&url)
        .header("Content-Type", "text/xml; charset=utf-8")
        .header("SOAPAction", "http://tempuri.org/CheckDeathStatus")
        .body(
            format!(
                r#"
                <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:api="http://tempuri.org/CheckDeathStatus"><soapenv:Header/><soapenv:Body><CheckDeathStatus xmlns="http://tempuri.org/"><pid>{}</pid><dob>{}</dob></CheckDeathStatus></soapenv:Body></soapenv:Envelope>
                "#, 
                csv_input.cid, s_dob
            )
        )
        .send().await;
    let duration = start.elapsed();
    // println!("borrower id {} elapse time {:?}", csv_input.cid, duration);

    let result1: Result<Response, ResponseError> = match result {
        
        Ok(res) => {
            let status_code = res.status().as_str().to_string();
            let body = res.text_with_charset("UTF-8").await.unwrap_or(String::from(""));

            Ok(Response {
                request_id: csv_input.cid.clone(),
                status_code: status_code,
                body: body,
                elapse: duration,
                csv_input: csv_input
            })
        },
        Err(error) => {
            Err(ResponseError {
                request_id: csv_input.cid.clone(),
                message: error.to_string(),
                elapse: duration,
                csv_input: csv_input
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
                elapse: duration,
                csv_input: csv_input
            })
        },
        Err(error) => {
            Err(ResponseError {
                request_id: csv_input.cid.clone(),
                message: error.to_string(),
                elapse: duration,
                csv_input: csv_input
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
    elapse: Duration,
    csv_input: CsvInput
}

#[derive(Debug)]
struct ResponseError {
    request_id: String,
    message: String,
    elapse: Duration,
    csv_input: CsvInput
}
