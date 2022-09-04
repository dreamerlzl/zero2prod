use poem::{get, handler, listener::TcpListener, web::Path, Route, Server};

#[handler]
pub fn hello(Path(name): Path<String>) -> String {
    format!("hello {}", name)
}

#[handler]
pub fn health_check() {
    // () -> 200 OK in poem
}

pub async fn run() -> Result<(), std::io::Error> {
    let app = Route::new()
        .at("/hello/:name", get(hello))
        .at("/health_check", get(health_check));
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
