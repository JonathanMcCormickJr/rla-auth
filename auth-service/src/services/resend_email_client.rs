use color_eyre::eyre::Result; // For improved error handling and reporting
use reqwest::{Client, Url}; // For making HTTP requests
use secrecy::{ExposeSecret, SecretString}; // For securely handling sensitive data

use crate::domain::{Email, EmailClient}; // Import domain-specific modules

// Define the ResendEmailClient struct
pub struct ResendEmailClient {
    http_client: Client, // HTTP client for making requests
    base_url: String, // Base URL for the email service
    sender: Email, // Email address of the sender
    api_key: SecretString, // Authorization token for the email service, wrapped in Secret for security
}

impl ResendEmailClient {
    // Constructor for creating a new ResendEmailClient instance
    pub fn new(
        base_url: String,
        sender: Email,
        api_key: SecretString,
        http_client: Client,
    ) -> Self {
        Self {
            http_client,
            base_url,
            sender,
            api_key,
        }
    }
}

#[async_trait::async_trait]
impl EmailClient for ResendEmailClient {
    #[tracing::instrument(name = "Sending email", skip_all)] // Trace this function, skipping logging its parameters
    async fn send_email(&self, recipient: &Email, subject: &str, content: &str) -> Result<()> {
        // Parse the base URL and join it with the email endpoint
        let base = Url::parse(&self.base_url)?;
        let url = base.join("/emails")?;

        // Create the request body for sending the email
        let request_body = SendEmailRequest {
            from: &format!("Auth Service <{}>", self.sender.as_ref().expose_secret()),
            to: &[recipient.as_ref().expose_secret()],
            subject,
            html: content,
        };

        // Build the HTTP POST request
        // Send the request and handle the response
        self.http_client
            .post(url)
            .header(
                RESEND_AUTH_HEADER,
                format!("Bearer {}", self.api_key.expose_secret()), // Securely expose the authorization token
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

// Constants for authorization header
const RESEND_AUTH_HEADER: &str = "Authorization";

// Define the structure of the email request body
// For more information about the request structure,
// see API docs: https://resend.com/docs/api-reference/emails/send-email
#[derive(serde::Serialize, Debug)]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a [&'a str],
    subject: &'a str,
    html: &'a str,
}

#[cfg(test)]
mod tests {
    use crate::utils::constants::test;

    use super::*;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    // Helper function to generate a test subject
    fn subject() -> String {
        Sentence(1..2).fake()
    }

    // Helper function to generate test content
    fn content() -> String {
        Paragraph(1..10).fake()
    }

    // Helper function to generate a test email
    fn email() -> Email {
        Email::parse(SecretString::new(SafeEmail().fake::<String>().into_boxed_str())).unwrap()
    }

    // Helper function to create a test email client
    fn email_client(base_url: String) -> ResendEmailClient {
        let http_client = Client::builder()
            .timeout(test::email_client::TIMEOUT)
            .build()
            .unwrap();
        ResendEmailClient::new(base_url, email(), SecretString::new(Faker.fake::<String>().into_boxed_str()), http_client)
    }

    // Custom matcher to validate the email request body
    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                body.get("from").is_some()
                    && body.get("to").is_some()
                    && body.get("subject").is_some()
                    && body.get("html").is_some()
            } else {
                false
            }
        }
    }

    // Test to ensure the email client sends the expected request
    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        // Set up the mock server to expect a specific request
        Mock::given(header_exists(RESEND_AUTH_HEADER))
            .and(header("Content-Type", "application/json"))
            .and(path("/emails"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Execute the send_email function and check the outcome
        let outcome = email_client
            .send_email(&email(), &subject(), &content())
            .await;

        assert!(outcome.is_ok());
    }

    // Test to handle server error responses
    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        // Set up the mock server to respond with a 500 error
        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Execute the send_email function and check the outcome
        let outcome = email_client
            .send_email(&email(), &subject(), &content())
            .await;

        assert!(outcome.is_err());
    }

    // Test to handle request timeouts
    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        // Set up the mock server to delay the response
        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180)); // 3 minutes delay
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        // Execute the send_email function and check the outcome
        let outcome = email_client
            .send_email(&email(), &subject(), &content())
            .await;

        assert!(outcome.is_err());
    }
}
