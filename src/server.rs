use tokio::net::{TcpListener, TcpStream};

use crate::{
    aof::{self, AofHandler},
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
        let mut store = Store::new();
        let aof = aof::AofHandler::new("appendonly.aof").await?;

        aof.restore(&mut store).await.unwrap();

        aof.start_worker();

        loop {
            let (conn, _) = listener.accept().await?;

            let store_clone = store.clone();
            let aof_clone = aof.clone();

            tokio::spawn(async move {
                process(conn, store_clone, aof_clone).await;
            });
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

async fn process(stream: TcpStream, store: Store, aof: AofHandler) {
    let mut connection = Connection::new(stream);
    loop {
        match connection.read_frame().await {
            Ok(frame) => {
                let response = execute_command(&frame, &store)
                    .await
                    .unwrap_or_else(|e| Frame::Error(format!("ERR {:?}", e)));

                if !matches!(response, Frame::Error(_)) {
                    aof.append_to_aof(&frame).await;
                }

                if connection.write_frame(&response).await.is_err() {
                    break;
                }
            }
            Err(Error::ConnectionClosed) => {
                // Client disconnected normally
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
