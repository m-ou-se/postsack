use core::num;
use eyre::{bail, Result};
use rayon::prelude::*;
use std::io::prelude::*;
use std::{io, path::PathBuf};
use thiserror;
use tracing_subscriber::EnvFilter;

use crossbeam_channel;
use std::path::Path;
use std::sync::{Arc, Mutex};

use std::{
    io::{stdout, Write},
    thread::sleep,
    time::Duration,
};

use crate::database::Database;

mod database;
mod emails;

#[derive(Debug, thiserror::Error)]
enum GmailDBError {
    #[error("Missing folder argument")]
    MissingFolder,
}
// ________________________________________________________
// Executed in  355.52 secs    fish           external
// usr time  121.22 secs   95.00 micros  121.22 secs
// sys time  456.33 secs  598.00 micros  456.33 secs

fn main() -> Result<()> {
    setup();
    let arguments: Vec<String> = std::env::args().collect();
    let folder = arguments.get(1).ok_or(GmailDBError::MissingFolder)?;
    let receiver = process_folder(&folder)?;
    let mut stdout = stdout();

    let mut total: Option<usize> = None;
    let mut counter = 0;
    let mut done = false;
    println!("Collecting Mails...");
    while done == false {
        if let Some(total) = total {
            for entry in receiver.try_iter() {
                let value = match entry {
                    Err(e) => {
                        panic!("{:?}", &e);
                    }
                    Ok(None) => {
                        done = true;
                        0
                    }
                    Ok(Some(n)) => n,
                };
                counter += value;
            }
            print!("\rProcessing {}/{}...", counter, total);
        } else {
            match receiver.recv()? {
                Err(e) => {
                    panic!("{:?}", &e);
                }
                Ok(Some(n)) => {
                    total = Some(n);
                }
                Ok(None) => done = true,
            };
        }
        stdout.flush().unwrap();
        sleep(Duration::from_millis(20));
    }
    println!();
    //process_email(&folder)?;
    Ok(())
}

//fn process_email(path: &str) -> Result<()> {
//    let entry = emails::RawEmailEntry::new(&path);
//    let mail = emails::read_email(&entry).unwrap();
//    Ok(())
//}

enum FolderProgress {
    Total(usize),
    Parsed,
}

fn process_folder(folder: &str) -> Result<crossbeam_channel::Receiver<Result<Option<usize>>>> {
    // We return the status
    let (tx, rx) = crossbeam_channel::bounded(100);
    let folder = folder.to_owned();

    std::thread::spawn(move || {
        let emails = match emails::read_folders(&folder) {
            Ok(n) => n,
            Err(e) => {
                tx.send(Err(e)).unwrap();
                return;
            }
        };
        let total = emails.len();

        tx.send(Ok(Some(total))).unwrap();

        println!("Done Loading {} emails", &total);

        let database = Database::new().expect("Expect a valid database");

        emails::process_emails(emails, Arc::new(Mutex::new(database)), tx.clone());

        /*use database::DBMessage;
        emails
            .emails
            .par_iter()
            //.iter()
            .map(|raw_mail| (raw_mail.path(), emails::read_email(&raw_mail)))
            .for_each(|(path, entry)| {
                tx.send(Ok(Some(1))).unwrap();
                if let Err(e) = match entry {
                    Ok(mail) => sender.send(DBMessage::Mail(mail)),
                    Err(e) => sender.send(DBMessage::Error(e, path)),
                } {
                    tracing::info!("Error Inserting into Database: {:?}", &e);
                }
            });*/

        //sender.send(database::DBMessage::Done).unwrap();
        //while !sender.is_empty() {
        //    //println!("left in sqlite: {}", sender.len());
        //    sleep(Duration::from_millis(50));
        //}
    });
    //tx.send(Ok(None)).unwrap();
    Ok(rx)
}

fn setup() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "error")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}
