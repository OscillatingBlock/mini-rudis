use mini_rudis::server::Server;

#[tokio::main]
async fn main() {
    let server = Server::new();
    let Ok(_) = server.run().await else {
        return;
    };
}
