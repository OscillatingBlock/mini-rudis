use bytes::Bytes;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

pub struct Connection {
    stream: BufReader<TcpStream>,
}

#[derive(Clone)]
pub enum Frame {
    Simple(String),
    Error(String),
    Integer(u64),
    Bulk(Bytes),
    Null,
    Array(Vec<Frame>),
    Hash(HashMap<String, Frame>),
}

#[derive(Debug)]
pub enum Error {
    ConnectionClosed,
    Incomplete,
    Other,
    Protocol(String),
    Io(std::io::Error),
}

// This allows us to use '?' on IO operations
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Protocol(msg) => write!(f, "{}", msg),
            Error::ConnectionClosed => write!(f, "Connection closed"),
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::Incomplete => write!(f, "Incomplete message"),
            _ => write!(f, "Internal server error"),
        }
    }
}

impl Connection {
    pub fn new(socket: TcpStream) -> Self {
        Connection {
            stream: BufReader::new(socket),
        }
    }

    pub async fn read_frame(&mut self) -> Result<Frame, Error> {
        let prefix = match self.stream.read_u8().await {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(Error::ConnectionClosed);
            }
            Err(e) => return Err(e.into()),
        };
        match prefix {
            b'+' => self.parse_simple_stream().await,
            b'$' => self.parse_bulk_stream().await,
            b'*' => self.parse_array_stream().await,
            b':' => self.parse_integer_stream().await,
            b'-' => self.parse_error_stream().await,

            _ => Result::Err(Error::Incomplete),
        }
    }

    async fn parse_simple_stream(&mut self) -> Result<Frame, Error> {
        let mut line = String::new();
        self.stream.read_line(&mut line).await?;

        if line.ends_with("\r\n") {
            line.truncate(line.len() - 2);
        }

        Ok(Frame::Simple(line))
    }

    async fn parse_bulk_stream(&mut self) -> Result<Frame, Error> {
        let mut len_line = String::new();
        self.stream.read_line(&mut len_line).await?;

        let len: i32 = len_line
            .trim()
            .parse()
            .map_err(|_| Error::Protocol("invalid bulk length".to_string()))?;

        if len == -1 {
            return Result::Ok(Frame::Null);
        }
        let mut data = vec![0; len as usize];
        self.stream.read_exact(&mut data).await?;

        //for after reading the line remove the remaining \r\n
        let mut len = String::new();
        self.stream.read_line(&mut len).await?;

        Ok(Frame::Bulk(Bytes::from(data)))
    }

    async fn parse_array_stream(&mut self) -> Result<Frame, Error> {
        let mut count_string = String::new();
        self.stream.read_line(&mut count_string).await?;
        let count: i32 = count_string
            .trim()
            .parse()
            .map_err(|_| Error::Protocol("invalid array length".to_string()))?;

        if count == -1 {
            return Ok(Frame::Null);
        }

        let mut results: Vec<Frame> = Vec::with_capacity(count as usize);

        for _ in 0..count {
            //shows recursion error
            // let frame = self.read_frame().await?;

            //to fix Create the future, box it, and pin it to the heap
            let frame = Box::pin(self.read_frame()).await?;
            results.push(frame);
        }
        Ok(Frame::Array(results))
    }

    async fn parse_integer_stream(&mut self) -> Result<Frame, Error> {
        let mut int_str = String::new();
        self.stream.read_line(&mut int_str).await?;
        let int: u64 = int_str
            .trim()
            .parse()
            .map_err(|_| Error::Protocol("invalid int".to_string()))?;

        Ok(Frame::Integer(int))
    }

    async fn parse_error_stream(&mut self) -> Result<Frame, Error> {
        let mut err_str = String::new();
        self.stream.read_line(&mut err_str).await?;
        Ok(Frame::Error(err_str))
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> Result<(), Error> {
        match frame {
            Frame::Simple(simple) => {
                self.stream.write_all(b"+").await?;
                self.stream.write_all(simple.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }

            Frame::Bulk(bulk) => {
                self.stream.write_all(b"$").await?;
                self.stream
                    .write_all(bulk.len().to_string().as_bytes())
                    .await?;
                self.stream.write_all(b"\r\n").await?;
                self.stream.write_all(bulk).await?;
                self.stream.write_all(b"\r\n").await?;
            }

            Frame::Error(err) => {
                self.stream.write_all(b"-").await?;
                self.stream.write_all(err.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }

            Frame::Integer(int) => {
                self.stream.write_all(b":").await?;
                self.stream.write_all(format!("{}", int).as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }

            Frame::Array(array) => {
                self.stream.write_all(b"*").await?;
                self.stream
                    .write_all(array.len().to_string().as_bytes())
                    .await?;
                self.stream.write_all(b"\r\n").await?;

                for item in array {
                    //use the pin method for recursion error fix
                    Box::pin(self.write_frame(item)).await?;
                }
            }

            Frame::Null => {
                self.stream.write_all(b"$-1\r\n").await?;
            }

            _ => return Err(Error::Other),
        }
        self.stream.flush().await?;
        Ok(())
    }
}
