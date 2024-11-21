//! Client builder, used by other flows to send requests and build
//! URLs.

use std::ops::Deref;

use oauth2::{
    http::{Method, Response},
    AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, HttpRequest, HttpResponse,
    RedirectUrl, TokenUrl,
};

use super::{Error, Result};

type BasicClient = oauth2::basic::BasicClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

/// Client builder, used by other flows to send requests and build
/// URLs.
#[derive(Clone, Debug)]
pub struct Client {
    inner: BasicClient,

    /// Hostname of the client's redirection endpoint.
    pub redirect_host: String,

    /// Port of the client's redirection endpoint.
    pub redirect_port: u16,
}

impl Client {
    pub fn new(
        client_id: impl ToString,
        client_secret: Option<impl ToString>,
        auth_url: impl ToString,
        token_url: impl ToString,
        redirect_scheme: impl ToString,
        redirect_host: impl ToString,
        redirect_port: impl Into<u16>,
    ) -> Result<Self> {
        let redirect_host = redirect_host.to_string();
        let redirect_port = redirect_port.into();

        let mut client = oauth2::basic::BasicClient::new(ClientId::new(client_id.to_string()))
            .set_auth_uri(AuthUrl::new(auth_url.to_string()).map_err(Error::BuildAuthUrlError)?)
            .set_token_uri(TokenUrl::new(token_url.to_string()).map_err(Error::BuildTokenUrlError)?)
            .set_redirect_uri({
                let scheme = redirect_scheme.to_string();
                RedirectUrl::new(format!("{scheme}://{redirect_host}:{redirect_port}"))
                    .map_err(Error::BuildRedirectUrlError)
            }?);

        if let Some(secret) = client_secret {
            client = client.set_client_secret(ClientSecret::new(secret.to_string()));
        }

        Ok(Self {
            inner: client,
            redirect_host,
            redirect_port,
        })
    }

    pub(crate) async fn send_oauth2_request(oauth2_request: HttpRequest) -> Result<HttpResponse> {
        let client = http::Client::new();

        let response = client
            .send(move |agent| match *oauth2_request.method() {
                Method::GET => {
                    let mut request = agent.get(&oauth2_request.uri().to_string());

                    for (key, val) in oauth2_request.headers() {
                        let Ok(val) = val.to_str() else {
                            continue;
                        };

                        request = request.header(key, val);
                    }

                    Ok(request.call()?)
                }
                Method::POST => {
                    let mut request = agent.post(&oauth2_request.uri().to_string());

                    for (key, val) in oauth2_request.headers() {
                        let Ok(val) = val.to_str() else {
                            continue;
                        };

                        request = request.header(key, val);
                    }

                    Ok(request.send(oauth2_request.body())?)
                }
                _ => unreachable!(),
            })
            .await?;

        let mut oauth2_response = Response::builder();

        for (key, val) in response.headers() {
            oauth2_response = oauth2_response.header(key, val);
        }

        let body = response
            .into_body()
            .read_to_vec()
            .map_err(http::Error::from)?;

        let oauth2_response = oauth2_response
            .body(body)
            .map_err(http::Error::from)
            .map_err(Error::ReadResponseBodyError)?;

        Ok(oauth2_response)
    }
}

impl Deref for Client {
    type Target = BasicClient;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
