use super::helpers::assert_is_redirect_to;
use crate::cookie_test;

cookie_test!(must_be_logged_in_to_access_the_board, [app] {
    let resp = app.get_admin_dashboard().await;
    assert_is_redirect_to(&resp, "/login");
});
