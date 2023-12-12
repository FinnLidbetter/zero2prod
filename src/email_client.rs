use reqwest::Client;
use secrecy::{ExposeSecret, Secret};

use crate::domain::SubscriberEmail;

pub struct EmailClient {
    http_client: Client,
    email_url: reqwest::Url,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
}
impl EmailClient {
    pub fn new(
        base_url: reqwest::Url,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout: std::time::Duration,
    ) -> Result<Self, String> {
        let email_url = base_url
            .join("/email")
            .map_err(|e| format!("Could not construct email url: {}.", e))?;
        let http_client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("Failed to build http client: {}", e))?;
        Ok(Self {
            http_client,
            email_url,
            sender,
            authorization_token,
        })
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };
        let builder = self
            .http_client
            .post(self.email_url.clone())
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body);
        builder.send().await?.error_for_status()?;
        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use claims::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                // If parsing failed do not match the request.
                false
            }
        }
    }

    /// Generate a random email subject.
    fn subject() -> String {
        Sentence(1..2).fake()
    }

    /// Generate a random email content.
    fn content() -> String {
        Paragraph(1..10).fake()
    }

    /// Generate a random subscriber email.
    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    /// Get a test instance of `EmailClient`
    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new(
            reqwest::Url::parse(&base_url).unwrap(),
            email(),
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _ = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        assert_err!(outcome);
    }
}
