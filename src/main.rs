use std::net::TcpListener;

use zero2prod::startup::run;

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8000")?;

    run(listener)?.await
}
