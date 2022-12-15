use reqwest::Client;
use secrecy::{ExposeSecret, Secret};
use serde::Serialize;

use crate::domain::Email;

#[derive(Clone)]
pub struct EmailClient {
    http_client: Client,
    api_base_url: String,
    sender: Email,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(
        api_base_url: String,
        sender: Email,
        authorization_token: String,
        timeout: std::time::Duration,
    ) -> Self {
        Self {
            http_client: Client::builder().timeout(timeout).build().unwrap(),
            api_base_url,
            sender,
            authorization_token: Secret::new(authorization_token),
        }
    }

    pub async fn hello(&self) {}

    pub async fn send_email(
        &self,
        recipient: &Email,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<reqwest::StatusCode, reqwest::Error> {
        let url = format!("{}/email", self.api_base_url);
        let request_body = SendEmailRequest {
            from: &self.sender,
            to: recipient,
            subject,
            html_body,
            text_body,
        };
        let resp = self
            .http_client
            .post(&url)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.status())
    }
}

// the fields are referenced from postmark
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a Email,
    to: &'a Email,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[cfg(test)]
mod tests {
    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::en::{Paragraph, Sentence},
        },
        Fake, Faker,
    };
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    use super::EmailClient;
    use crate::domain::Email;

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result = serde_json::from_slice::<serde_json::Value>(&request.body);
            if let Ok(body) = result {
                // dbg!(&body);
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    #[ignore]
    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1) // only 1 request is sent during this test
            .mount(&mock_server)
            .await;

        let subscriber_email = Email::parse(SafeEmail().fake::<String>()).unwrap();
        let subject = subject();
        let content = content();
        let outcome = email_client
            .send_email(&subscriber_email, &subject, &content, &content)
            .await;
        dbg!(&outcome);
        assert!(outcome.is_ok());
    }

    #[ignore]
    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(502))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = email();
        let subject = subject();
        let content = content();
        let outcome = email_client
            .send_email(&subscriber_email, &subject, &content, &content)
            .await;
        assert!(outcome.is_err());
    }

    #[ignore]
    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert!(outcome.is_err());
    }

    fn email_client(base_url: String) -> EmailClient {
        let sender = Email::parse(SafeEmail().fake::<String>()).unwrap();
        EmailClient::new(
            base_url,
            sender,
            Faker.fake(),
            std::time::Duration::from_millis(200),
        )
    }

    fn email() -> Email {
        Email::parse(SafeEmail().fake::<String>()).unwrap()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }
}
