use std::{
    future::Future,
    io,
    ops::Deref,
    path::{Path, PathBuf},
    pin::Pin,
};

use google_drive3::{
    hyper::{self, client::HttpConnector},
    hyper_rustls::{HttpsConnector, HttpsConnectorBuilder},
    oauth2::{self, authenticator::Authenticator, authenticator_delegate::InstalledFlowDelegate},
    DriveHub,
};

use crate::app_config;

pub struct HubConfig {
    pub secret: oauth2::ApplicationSecret,
    pub tokens_path: PathBuf,
}

pub struct Hub(DriveHub<HttpsConnector<HttpConnector>>);

impl Deref for Hub {
    type Target = DriveHub<HttpsConnector<HttpConnector>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hub {
    pub fn new(auth: Auth) -> io::Result<Hub> {
        let connector = HttpsConnectorBuilder::new()
            .with_native_roots()?
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build();

        let http_client = hyper::Client::builder().build(connector);

        Ok(Hub(google_drive3::DriveHub::new(http_client, auth.0)))
    }
}

pub struct Auth(pub Authenticator<HttpsConnector<HttpConnector>>);

impl Deref for Auth {
    type Target = Authenticator<HttpsConnector<HttpConnector>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Auth {
    pub async fn new(config: &app_config::Secret, tokens_path: &Path) -> Result<Auth, io::Error> {
        let secret = oauth2_secret(config);
        let delegate = Box::new(AuthDelegate);

        let auth = oauth2::InstalledFlowAuthenticator::builder(
            secret,
            oauth2::InstalledFlowReturnMethod::HTTPPortRedirect(8085),
        )
        .persist_tokens_to_disk(tokens_path)
        .flow_delegate(delegate)
        .build()
        .await?;

        Ok(Auth(auth))
    }
}

fn oauth2_secret(config: &app_config::Secret) -> oauth2::ApplicationSecret {
    oauth2::ApplicationSecret {
        client_id: config.client_id.clone(),
        client_secret: config.client_secret.clone(),
        token_uri: String::from("https://oauth2.googleapis.com/token"),
        auth_uri: String::from("https://accounts.google.com/o/oauth2/auth"),
        redirect_uris: vec![String::from("urn:ietf:wg:oauth:2.0:oob")],
        project_id: None,
        client_email: None,
        auth_provider_x509_cert_url: Some(String::from(
            "https://www.googleapis.com/oauth2/v1/certs",
        )),
        client_x509_cert_url: None,
    }
}

struct AuthDelegate;

impl InstalledFlowDelegate for AuthDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        _need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            println!();
            println!();
            println!("Gdrive requires permissions to manage your files on Google Drive.");
            println!("Open the url in your browser and follow the instructions:");
            println!("{url}");
            Ok(String::new())
        })
    }
}
