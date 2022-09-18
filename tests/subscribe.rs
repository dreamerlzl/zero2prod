use poem::http::StatusCode;
use poem::test::TestClient;

use zero2prod::configuration::get_test_configuration;
use zero2prod::routes::default_route;

#[tokio::test]
async fn subscribe_returns_400_for_invalid_data() {
    let conf = get_test_configuration("config/test").expect("fail to get conf");
    let app = default_route(conf).await;
    let cli = TestClient::new(app);
    let invalid_data = ["", "name=lzl", "email=aaa", "name=lzl&email=aaa", "foobar"];

    for data in invalid_data.into_iter() {
        let resp = cli
            .post("/subscription")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(data)
            .send()
            .await;
        resp.assert_status(StatusCode::BAD_REQUEST);
    }
}
