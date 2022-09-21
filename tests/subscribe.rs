use poem::http::StatusCode;
use poem::test::TestClient;
use poem::Route;
use tracing_test::traced_test;

use zero2prod::configuration::get_test_configuration;
use zero2prod::routes::default_route;

async fn get_client() -> TestClient<Route> {
    let conf = get_test_configuration("config/test").expect("fail to get conf");
    let app = default_route(conf).await;
    TestClient::new(app)
}

#[tokio::test]
#[traced_test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let cli = get_client().await;
    let valid_data = ["user=lzl&email=lzl2@gmail.com"];

    for data in valid_data.into_iter() {
        let mut resp = cli
            .post("/subscription")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(data)
            .send()
            .await;
        // dbg!(resp.json().await.value());
        resp.assert_status(StatusCode::OK);
    }
}

#[tokio::test]
#[traced_test]
async fn subscribe_returns_400_for_invalid_data() {
    let cli = get_client().await;
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
