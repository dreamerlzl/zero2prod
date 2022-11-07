use poem::{test::TestClient, Route};
use zero2prod::routes::health::health_check;

#[tokio::test]
async fn test_health_check() {
    let app = Route::new().at("/health_check", health_check);
    let cli = TestClient::new(app);

    let resp = cli.get("/health_check").send().await;
    resp.assert_status_is_ok();
}
