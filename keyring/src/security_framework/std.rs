use std::string::FromUtf8Error;

use secrecy::ExposeSecret;
use security_framework::{
    base::Error as SecurityFrameworkError,
    passwords::{delete_generic_password, get_generic_password, set_generic_password},
};
use thiserror::Error;

use crate::{
    event::KeyringEvent,
    state::{KeyringState, KeyringState2},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot read secret from OSX keychain at {1}:{2}")]
    ReadSecretError(#[source] SecurityFrameworkError, String, String),
    #[error("cannot update secret from OSX keychain at {1}:{2}")]
    UpdateSecretError(#[source] SecurityFrameworkError, String, String),
    #[error("cannot delete secret from OSX keychain at {1}:{2}")]
    DeleteSecretError(#[source] SecurityFrameworkError, String, String),
    #[error("cannot parse secret as UTF-8 string")]
    ParseSecretError(#[from] FromUtf8Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn progress(state: &mut KeyringState2) -> Result<Option<KeyringEvent>> {
    match state.next() {
        None => Ok(None),
        Some(KeyringState::ReadSecret) => {
            let secret = get_generic_password(&state.service, &state.account).map_err(|err| {
                Error::UpdateSecretError(err, state.service.clone(), state.account.clone())
            })?;
            let secret = String::from_utf8(secret)?;
            Ok(Some(KeyringEvent::SecretRead(secret.into())))
        }
        Some(KeyringState::UpdateSecret(secret)) => {
            let secret = secret.expose_secret().as_bytes();
            set_generic_password(&state.service, &state.account, secret).map_err(|err| {
                Error::UpdateSecretError(err, state.service.clone(), state.account.clone())
            })?;
            Ok(Some(KeyringEvent::SecretUpdated))
        }
        Some(KeyringState::DeleteSecret) => {
            delete_generic_password(&state.service, &state.account).map_err(|err| {
                Error::DeleteSecretError(err, state.service.clone(), state.account.clone())
            })?;
            Ok(Some(KeyringEvent::SecretDeleted))
        }
    }
}
