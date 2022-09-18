use poem::handler;

#[handler]
pub fn health_check() {
    // () -> 200 OK in poem
}
