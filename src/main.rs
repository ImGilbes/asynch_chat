// rust compiler doesn't know how to interleave asynch computations
// future values are items that will have a value in the future, eventually
// futures can be in a pending or complete state
// rust knows how to generate futures but not how to run them+
// to fix this -> use a procedural macro available from tokyo
#![allow(dead_code, unused)]
use std::{net::SocketAddr};
use std::io::{Error, ErrorKind};
use tokio::{
    net::TcpListener, 
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader}, sync::broadcast
};

use chrono::{Datelike, Timelike, Utc};
use chrono_tz::Europe::Rome;

#[tokio::main]
async fn main(){
    

    let my_addr = "0.0.0.0:7878".parse::<SocketAddr>().unwrap();

    //first make tcp echo server
    let listener = TcpListener::bind(my_addr).await.unwrap();
    //await == suspend this action until the future is ready

    //async methods avoid blocking at trhead level, but block at task level

    let (tx, mut rx) = broadcast::channel::<(String, SocketAddr)>(10); 
    loop{
        let (mut socket, client_addr) = listener.accept().await.unwrap();

        //these lines are needed to avoid a compiler errorr: use of moved value
        let tx = tx.clone();
        let mut rx = tx.subscribe();

        // async block! spawn a new thread for each connecting client!
        tokio::spawn(async move {
            let (sock_read, mut sock_write) = socket.split();

            let mut reader = BufReader::new(sock_read);

            let mut line = String::new();
            let mut read_len : usize = 0;
            let mut username = String::new();

            while read_len == 0{
                line.clear();
                sock_write.write_all("Insert username: ".as_bytes()).await.unwrap();
                read_len = reader.read_line(&mut line).await.unwrap();
                if read_len > 0{
                    username = line.clone();
                    username = username.trim_end().to_string();
                    line.clear();
                }
            }

            let msg = format!("\r\nWelcome to our house chat {}!\r\n", username);
            sock_write.write_all(msg.as_bytes()).await.unwrap();
            sock_write.write_all("\r\nYou can now type any message.\r\nPress Enter and it will be sent to the other people connected!\r\n\r\n".as_bytes()).await.unwrap();
            
            let msg = format!("{} Has connected to the chat! ({})\r\n", username, full_local_time_string());
            tx.send((msg.clone(), client_addr)).unwrap();

            loop {
                tokio::select! {

                    result = reader.read_line(&mut line) =>{
                        match result{
                            Ok(r) => {
                                if r == 0{ //can do something btter here
                                    let msg = format!("{} has disconnected ({})\r\n", username, full_local_time_string());
                                    tx.send((msg.clone(), client_addr)).unwrap();
        
                                    break; //if the client has disconnected
                                }
                                let msg = line.clone();

                                let msg = format!("({}) {}: {}", simple_local_time_string(), username, msg);
                                tx.send((msg.clone(), client_addr)).unwrap(); // it also broadcasts to itself

                                line.clear();  //clean the input buffer to avoid repeated messages
                            }
                            Err(e) => {
                                match e.kind(){
                                    ErrorKind::ConnectionAborted => {
                                        let msg = format!("{} has disconnected ({})\r\n", username, full_local_time_string());
                                        tx.send((msg.clone(), client_addr)).unwrap();

                                        break;
                                    }
                                    _ => {
                                        println!("weird error: {}", e.to_string());
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    result = rx.recv() => {
                        let (msg, recv_addr) = result.unwrap();
                        
                        if recv_addr != client_addr {
                            sock_write.write_all(msg.as_bytes()).await.unwrap();
                        }
                    }
                }

            }
        });
    }
}

fn simple_local_time_string() -> String{
    let now = Utc::now();
    let now = now.with_timezone(&Rome);
    let time = now.format("%T").to_string();
    time
}

fn full_local_time_string() -> String{
    let now = Utc::now();  
    let now = now.with_timezone(&Rome);
    let time = now.format("%a %b %e %T %Y").to_string();
    time
}
