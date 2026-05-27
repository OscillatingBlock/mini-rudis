use crate::connection::{Connection, Error, Frame};
use crate::handler;
use crate::handler::execute_command;
use crate::store::Store;
use std::io::Write;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

pub struct AofHandler {
    inner: Arc<Mutex<AofInner>>,
}

struct AofInner {
    file: File,
    buffer: Vec<u8>,
}

impl Clone for AofHandler {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl AofHandler {
    pub async fn new(path: &str) -> Result<Self, std::io::Error> {
        let file = tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .await?;

        Ok(Self {
            inner: Arc::new(Mutex::new(AofInner {
                file,
                buffer: Vec::new(),
            })),
        })
    }

    pub async fn append_to_aof(&self, frame: &Frame) {
        let Frame::Array(frames) = frame else {
            return;
        };
        let Some(first) = frames.first() else {
            return;
        };
        let Some(cmd_name) = handler::to_string(first) else {
            return;
        };

        match cmd_name.to_uppercase().as_str() {
            "SET" | "HSET" => {
                let mut inner = self.inner.lock().await;
                let _ = inner.extend_buffer(frame);
            }

            _ => {}
        }
    }

    // Inside your implementation
    pub fn start_worker(&self) {
        let inner_clone = self.inner.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                let mut inner = inner_clone.lock().await;
                if !inner.buffer.is_empty() {
                    //this results in error since ew are bowwoing immutably an mutably both , which
                    //is not allowed
                    // if let Ok(_) = inner.file.write_all(&inner.buffer).await {
                    //     let _ = inner.file.sync_all().await;
                    //     inner.buffer.clear();
                    // }
                    //
                    // steal the buffer and replace it with empty one , creating a new variable
                    // which is immutably borrowed
                    let data_to_write = std::mem::take(&mut inner.buffer);

                    //now inner is only borrowed mutably for file
                    if inner.file.write_all(&data_to_write).await.is_ok() {
                        let _ = inner.file.sync_all().await;
                    } else {
                        inner.buffer.extend_from_slice(&data_to_write);
                    }
                }
            }
        });
    }

    pub async fn restore(&self, store: &mut Store) -> Result<(), Error> {
        if !std::path::Path::new("appendonly.aof").exists() {
            return Ok(());
        }
        println!("starting AOF recovery...");

        let file = File::open("appendonly.aof").await?;
        let mut connection = Connection::new(file);

        loop {
            match connection.read_frame().await {
                Ok(frame) => {
                    let _ = execute_command(&frame, store).await;
                }

                Err(Error::ConnectionClosed) => {
                    break;
                }
                Err(e) => {
                    // Something actually went wrong (Protocol error, timeout, etc.)
                    println!("Protocol error: {:?}", e);
                    return Err(e);
                }
            }
        }
        println!("AOF recovery complete.");
        Ok(())
    }
}

impl AofInner {
    //since we are writing to buffer which is in memory and not in disk ,
    //this function does not need to be async and we can use std io write
    fn extend_buffer(&mut self, frame: &Frame) -> Result<(), Error> {
        // This forces the use of the standard synchronous Write trait
        let buf = &mut self.buffer;

        match frame {
            Frame::Simple(simple) => {
                Write::write_all(buf, b"+")?;
                Write::write_all(buf, simple.as_bytes())?;
                Write::write_all(buf, b"\r\n")?;
            }

            Frame::Bulk(bulk) => {
                Write::write_all(buf, b"$")?;
                Write::write_all(buf, bulk.len().to_string().as_bytes())?;
                Write::write_all(buf, b"\r\n")?;
                Write::write_all(buf, bulk)?;
                Write::write_all(buf, b"\r\n")?;
            }

            Frame::Error(err) => {
                Write::write_all(buf, b"-")?;
                Write::write_all(buf, err.as_bytes())?;
                Write::write_all(buf, b"\r\n")?;
            }

            Frame::Integer(int) => {
                Write::write_all(buf, b":")?;
                Write::write_all(buf, int.to_string().as_bytes())?;
                Write::write_all(buf, b"\r\n")?;
            }

            Frame::Array(array) => {
                Write::write_all(buf, b"*")?;
                Write::write_all(buf, array.len().to_string().as_bytes())?;
                Write::write_all(buf, b"\r\n")?;

                for item in array {
                    self.extend_buffer(item)?;
                }
            }

            Frame::Null => {
                Write::write_all(buf, b"$-1\r\n")?;
            }

            _ => return Err(Error::Other),
        }
        Ok(())
    }
}
