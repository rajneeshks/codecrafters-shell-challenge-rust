use crate::commands::array;
use crate::commands::incoming;
use crate::commands::ss;
use crate::repl::repl;
use crate::store::db;
use bytes::BytesMut;
use std::io::Write;
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct SetOptions {
    pub expiry_in_ms: u64,
}

impl SetOptions {
    fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

pub fn set_handler<'a, 'b>(
    datain: &incoming::Incoming,
    stream: &mut TcpStream,
    db: &Arc<db::DB>,
    replcfg: &Arc<repl::ReplicationConfig>,
    tx_ch: &Sender<BytesMut>,
) -> std::io::Result<()> {
    let cmd = &datain.command;
    let mut response = String::new(); //("*\r\n");
    let key_option = array::get_nth_arg(cmd, 1);
    let val_option = array::get_nth_arg(cmd, 2);
    if key_option.is_none() || val_option.is_none() {
        return ss::invalid(datain, stream, db, replcfg, tx_ch);
    }
    let key = key_option.unwrap();
    let val = val_option.unwrap();
    let mut options = SetOptions::new();
    // search for expiry option
    let mut argidx = 3;
    let px_option = "px".to_string();
    while let Some(opt) = array::get_nth_arg(cmd, argidx) {
        match opt {
            px_option => {
                argidx += 1;
                if let Some(expiry) = array::get_nth_arg(cmd, argidx) {
                    if let Ok(expiry_ms) = expiry.parse::<u64>() {
                        options.expiry_in_ms = expiry_ms;
                    }
                }
            }
            _ => {}
        };
        argidx += 1;
    }

    if let Ok(_o) = db.add(key.clone(), val.clone(), &options) {
        // replicate it now
        match tx_ch.send(datain.buf.clone()) {
            Ok(_) => {}
            Err(e) => println!("error replicating command, error: {:?}", e),
        };
        let _ = std::fmt::write(&mut response, format_args!("+OK\r\n"));
    } else {
        let _ = std::fmt::write(&mut response, format_args!("-Error writing to DB\r\n"));
    }
    stream.write_all(response.as_bytes())
}

pub fn get_handler(
    datain: &incoming::Incoming,
    stream: &mut TcpStream,
    db: &Arc<db::DB>,
    replcfg: &Arc<repl::ReplicationConfig>,
    tx_ch: &Sender<BytesMut>,
) -> std::io::Result<()> {
    let cmd = &datain.command;
    let mut response = String::new(); //("*\r\n");
    if let Some(key) = array::get_nth_arg(cmd, 1) {
        if let Some(val) = db.get(key) {
            let _ = std::fmt::write(
                &mut response,
                format_args!("${}\r\n{}\r\n", val.chars().count(), val),
            );
        } else {
            // did not find
            let _ = std::fmt::write(&mut response, format_args!("$-1\r\n"));
        }
    } else {
        return ss::invalid(datain, stream, db, replcfg, tx_ch);
    }
    stream.write_all(response.as_bytes())
}
