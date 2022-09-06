use poem::test::TestClient;
use poem::http::StatusCode;

use zero2prod::default_route;

#[tokio::test]
async fn subscribe_returns_400_for_invalid_data() {
    let app = default_route();
    let cli = TestClient::new(app);
    let invalid_data = [
        "",
        "name=lzl",
        "email=aaa",
        "name=lzl&email=aaa",
        "foobar",
    ];

    for data in invalid_data.into_iter() {
        let resp = cli.post("/subscription").header("Content-Type", "application/x-www-form-urlencoded").body(data).send().await;
        resp.assert_status(StatusCode::BAD_REQUEST);
    }
}