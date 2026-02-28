use tokio::net::{TcpListener, TcpStream};

use crate::{
    connection::{Connection, Error, Frame},
    handler::execute_command,
    store::Store,
};

pub struct Server {}

impl Server {
    pub fn new() -> Self {
        Server {}
    }
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("127.0.0.1:6379").await?;
        println!("Mini-Redis server running on port 6379");
        let store = Store::new();

        loop {
            let (conn, _) = listener.accept().await?;
            let store_clone = store.clone();
            tokio::spawn(async move {
                process(conn, store_clone).await;
            });
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

async fn process(stream: TcpStream, store: Store) {
    let mut connection = Connection::new(stream);
    loop {
        match connection.read_frame().await {
            Ok(frame) => {
                let response = execute_command(frame, store.clone())
                    .await
                    .unwrap_or_else(|e| Frame::Error(format!("ERR {:?}", e)));

                if connection.write_frame(&response).await.is_err() {
                    break; // Can't write? Client is gone.
                }
            }
            Err(Error::ConnectionClosed) => {
                // Client disconnected normally (e.g., closed the terminal)
                break;
            }
            Err(e) => {
                // Something actually went wrong (Protocol error, timeout, etc.)
                eprintln!("Protocol error: {:?}", e);
                break;
            }
        }
    }
    println!("Connection closed.");
}
