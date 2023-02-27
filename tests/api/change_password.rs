use uuid::Uuid;

use super::helpers::assert_is_redirect_to;
use crate::cookie_test;

cookie_test!(must_be_logged_in_to_see_the_change_password_form, [app] {
    let resp = app.get_change_password().await;
    assert_is_redirect_to(&resp, "/login");
});

cookie_test!(must_be_logged_in_to_change_your_password, [app] {
    let new_password = Uuid::new_v4().to_string();
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;
    assert_is_redirect_to(&resp, "/login");
});

cookie_test!(new_password_fields_must_match, [app]{
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();

    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    })).await?;

    let resp = app.post_change_password(&serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": &new_password,
        "new_password_check": &another_new_password,
    })).await;
    assert_is_redirect_to(&resp, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>You entered two different new passwords - the field values must match</i></p>"), "{}", html_page);
});

cookie_test!(current_password_must_be_valid, [app]{
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    })).await?;

    let resp = app.post_change_password(&serde_json::json!({
        "current_password": &wrong_password,
        "new_password": &new_password,
        "new_password_check": &new_password,
    })).await;

    assert_is_redirect_to(&resp, "/admin/password");
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>The current password is incorrect</i></p>"), "{}", html_page);
});
