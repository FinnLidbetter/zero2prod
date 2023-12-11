//! main.rs

use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod::configuration::get_configuration;
use zero2prod::email_client::EmailClient;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPool::connect_lazy_with(configuration.database.with_db());
    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address.");
    let email_base_url =
        reqwest::Url::parse(&configuration.email_client.base_url).expect("Invalid base email url.");
    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(
        email_base_url,
        sender_email,
        configuration.email_client.authorization_token,
        timeout,
    )
    .expect("Failed to construct email client.");
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool, email_client)?.await
}
