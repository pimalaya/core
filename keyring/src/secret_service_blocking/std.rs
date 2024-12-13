use std::{collections::HashMap, fmt, time::Duration};

use dbus::{
    arg::{PropMap, RefArg, Variant},
    blocking::{Connection, Proxy},
    Path,
};
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;

use crate::{
    event::KeyringEvent,
    state::{KeyringState, KeyringState2},
};

use super::api::{OrgFreedesktopSecretCollection, OrgFreedesktopSecretService};

static BUS_NAME: &str = "org.freedesktop.secrets";
static BUS_PATH: &str = "/org/freedesktop/secrets";

static TIMEOUT: Duration = Duration::from_secs(2);

static SS_ITEM_LABEL: &str = "org.freedesktop.Secret.Item.Label";
static SS_ITEM_ATTRIBUTES: &str = "org.freedesktop.Secret.Item.Attributes";

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot create D-Bus connection")]
    CreateSessionError(#[source] dbus::Error),
    #[error("cannot open D-Bus session")]
    OpenSessionError(#[source] dbus::Error),
    #[error("cannot get default secret service collection")]
    GetDefaultCollectionError(#[source] dbus::Error),
    #[error("cannot get session secret service collection")]
    GetSessionCollectionError(#[source] dbus::Error),
    #[error("cannot get secret service collections")]
    GetCollectionsError(#[source] dbus::Error),
    #[error("cannot create default secret service collection")]
    CreateDefaultCollectionError(#[source] dbus::Error),
    #[error("cannot create secret service collection item")]
    CreateItemError(#[source] dbus::Error),
}

pub type Result<T> = ::std::result::Result<T, Error>;

pub struct SecretService {
    connection: Connection,
    session_path: Path<'static>,
}

impl fmt::Debug for SecretService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretService")
            .field("session_path", &self.session_path)
            .finish_non_exhaustive()
    }
}

impl SecretService {
    pub fn connect() -> Result<Self> {
        let connection = Connection::new_session().map_err(Error::CreateSessionError)?;
        let proxy = connection.with_proxy(BUS_NAME, BUS_PATH, TIMEOUT);

        let bytes_arg = Box::new(String::new()) as Box<dyn RefArg>;
        let (_, session_path) = proxy
            .open_session("plain", Variant(bytes_arg))
            .map_err(Error::OpenSessionError)?;

        Ok(Self {
            connection,
            session_path,
        })
    }

    pub fn get_default_collection(&self) -> Result<Collection<'_>> {
        let proxy = self.connection.with_proxy(BUS_NAME, BUS_PATH, TIMEOUT);
        let empty_path = Path::default();

        let collection_path = proxy
            .read_alias("default")
            .map_err(Error::GetDefaultCollectionError)?;

        if collection_path != empty_path {
            return Ok(Collection::new(self, collection_path));
        }

        let collection_path = proxy
            .read_alias("session")
            .map_err(Error::GetSessionCollectionError)?;

        if collection_path != empty_path {
            return Ok(Collection::new(self, collection_path));
        }

        let collections_path = proxy.collections().map_err(Error::GetCollectionsError)?;

        match collections_path.into_iter().next() {
            Some(collection_path) => Ok(Collection::new(self, collection_path)),
            None => {
                let props: PropMap = HashMap::from_iter(Some((
                    "org.freedesktop.Secret.Collection.Label".into(),
                    Variant(Box::new(String::from("default")) as Box<dyn RefArg>),
                )));

                let (collection_path, _prompt_path) = proxy
                    .create_collection(props, "default")
                    .map_err(Error::CreateDefaultCollectionError)?;

                let collection_path = if collection_path == empty_path {
                    // no creation path, so prompt
                    todo!()
                } else {
                    collection_path
                };

                Ok(Collection::new(self, collection_path))
            }
        }
    }
}

#[derive(Debug)]
pub struct Collection<'a> {
    service: &'a SecretService,
    path: Path<'a>,
}

impl<'a> Collection<'a> {
    pub fn new(service: &'a SecretService, path: Path<'a>) -> Self {
        Self { service, path }
    }

    pub fn proxy(&self) -> Proxy<'_, &'a Connection> {
        self.service
            .connection
            .with_proxy(BUS_NAME, &self.path, TIMEOUT)
    }

    pub fn create_item(
        &self,
        service: impl ToString,
        account: impl ToString,
        secret: impl Into<SecretString>,
    ) -> Result<Item<'_>> {
        let secret = secret.into().expose_secret().as_bytes().to_vec();
        let label = Box::new(service.to_string() + ":" + &account.to_string());
        let attrs: Box<HashMap<String, String>> = Box::new(HashMap::from_iter([
            (String::from("service"), service.to_string()),
            (String::from("account"), account.to_string()),
        ]));

        let mut props: PropMap = PropMap::new();
        props.insert(SS_ITEM_LABEL.into(), Variant(label));
        props.insert(SS_ITEM_ATTRIBUTES.into(), Variant(attrs));

        let session_path = self.service.session_path.clone();
        let secret = (session_path, vec![], secret, "text/plain");
        let (item_path, _prompt_path) = self
            .proxy()
            .create_item(props, secret, true)
            .map_err(Error::CreateItemError)?;

        let item_path = if item_path == Path::default() {
            // no creation path, so prompt
            todo!()
        } else {
            item_path
        };

        Ok(Item::new(&self.service, item_path))
    }
}

#[derive(Debug)]
pub struct Item<'a> {
    service: &'a SecretService,
    path: Path<'a>,
}

impl<'a> Item<'a> {
    pub fn new(service: &'a SecretService, path: Path<'a>) -> Self {
        Self { service, path }
    }

    pub fn proxy(&self) -> Proxy<'_, &'a Connection> {
        self.service
            .connection
            .with_proxy(BUS_NAME, &self.path, TIMEOUT)
    }
}

pub fn progress(state: &mut KeyringState2) -> Result<Option<KeyringEvent>> {
    match state.next() {
        None => Ok(None),
        Some(KeyringState::ReadSecret) => {
            todo!()
            // let ss = SecretService::connect()?;
            // let collection = ss.get_default_collection()?;
            // collection.create_item(state.service.clone(), state.account.clone(), )

            // let (unlocked, _locked) = OrgFreedesktopSecretService::search_items(
            //     &ss.proxy(),
            //     HashMap::from_iter([
            //         ("service", state.service.as_str()),
            //         ("account", state.account.as_str()),
            //     ]),
            // )?;
            // let item = unlocked.into_iter().next().unwrap();
            // let (_path, _salt, secret, _mime) =
            //     ss.with_proxy(item).get_secret(ss.session_path.clone())?;
            // let secret = String::from_utf8(secret).unwrap();
            // Ok(Some(KeyringEvent::SecretRead(secret.into())))
        }
        Some(KeyringState::UpdateSecret(secret)) => {
            let ss = SecretService::connect()?;
            let collection = ss.get_default_collection()?;
            println!("collection: {collection:?}");

            let item =
                collection.create_item(state.service.clone(), state.account.clone(), secret)?;
            println!("item: {item:?}");

            Ok(Some(KeyringEvent::SecretUpdated))
        }
        Some(KeyringState::DeleteSecret) => {
            todo!()
        }
    }
}
