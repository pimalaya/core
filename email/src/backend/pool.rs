//! # Backend pool
//!
//! A [`BackendPool`] allows you to execute batches of features in
//! parallel.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::oneshot::{Receiver, Sender};

use super::{
    context::{BackendContext, BackendContextBuilder},
    feature::BackendFeature,
    AsyncTryIntoBackendFeatures, BackendBuilder, Error,
};
use crate::{
    account::config::{AccountConfig, HasAccountConfig},
    envelope::{
        get::GetEnvelope,
        list::{ListEnvelopes, ListEnvelopesOptions},
        thread::ThreadEnvelopes,
        watch::WatchEnvelopes,
        Envelope, Envelopes, Id, SingleId, ThreadedEnvelopes,
    },
    flag::{add::AddFlags, remove::RemoveFlags, set::SetFlags, Flags},
    folder::{
        add::AddFolder, delete::DeleteFolder, expunge::ExpungeFolder, list::ListFolders,
        purge::PurgeFolder, Folders,
    },
    message::{
        add::AddMessage, copy::CopyMessages, delete::DeleteMessages, get::GetMessages,
        peek::PeekMessages, r#move::MoveMessages, send::SendMessage, Messages,
    },
    thread_pool::{ThreadPool, ThreadPoolBuilder, ThreadPoolContext, ThreadPoolContextBuilder},
    AnyResult,
};

/// The backend pool.
///
/// This implementation owns a pool of context, and backend features
/// are executed by the first available context.
///
/// This implementation is useful when you need to call a batch of
/// features, in parallel.
pub struct BackendPool<C: BackendContext> {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,
    /// The backend context pool.
    pub pool: ThreadPool<C>,

    /// The add folder backend feature.
    pub add_folder: Option<BackendFeature<C, dyn AddFolder>>,
    /// The list folders backend feature.
    pub list_folders: Option<BackendFeature<C, dyn ListFolders>>,
    /// The expunge folder backend feature.
    pub expunge_folder: Option<BackendFeature<C, dyn ExpungeFolder>>,
    /// The purge folder backend feature.
    pub purge_folder: Option<BackendFeature<C, dyn PurgeFolder>>,
    /// The delete folder backend feature.
    pub delete_folder: Option<BackendFeature<C, dyn DeleteFolder>>,

    /// The get envelope backend feature.
    pub get_envelope: Option<BackendFeature<C, dyn GetEnvelope>>,
    /// The list envelopes backend feature.
    pub list_envelopes: Option<BackendFeature<C, dyn ListEnvelopes>>,
    /// The thread envelopes backend feature.
    pub thread_envelopes: Option<BackendFeature<C, dyn ThreadEnvelopes>>,
    /// The watch envelopes backend feature.
    pub watch_envelopes: Option<BackendFeature<C, dyn WatchEnvelopes>>,

    /// The add flags backend feature.
    pub add_flags: Option<BackendFeature<C, dyn AddFlags>>,
    /// The set flags backend feature.
    pub set_flags: Option<BackendFeature<C, dyn SetFlags>>,
    /// The remove flags backend feature.
    pub remove_flags: Option<BackendFeature<C, dyn RemoveFlags>>,

    /// The add message backend feature.
    pub add_message: Option<BackendFeature<C, dyn AddMessage>>,
    /// The send message backend feature.
    pub send_message: Option<BackendFeature<C, dyn SendMessage>>,
    /// The peek messages backend feature.
    pub peek_messages: Option<BackendFeature<C, dyn PeekMessages>>,
    /// The get messages backend feature.
    pub get_messages: Option<BackendFeature<C, dyn GetMessages>>,
    /// The copy messages backend feature.
    pub copy_messages: Option<BackendFeature<C, dyn CopyMessages>>,
    /// The move messages backend feature.
    pub move_messages: Option<BackendFeature<C, dyn MoveMessages>>,
    /// The delete messages backend feature.
    pub delete_messages: Option<BackendFeature<C, dyn DeleteMessages>>,
}

impl<C: BackendContext> HasAccountConfig for BackendPool<C> {
    fn account_config(&self) -> &AccountConfig {
        &self.account_config
    }
}

#[async_trait]
impl<C: BackendContext + 'static> AddFolder for BackendPool<C> {
    async fn add_folder(&self, folder: &str) -> AnyResult<()> {
        let folder = folder.to_owned();
        let feature = self
            .add_folder
            .clone()
            .ok_or(Error::AddFolderNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::AddFolderNotAvailableError)?
                    .add_folder(&folder)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> ListFolders for BackendPool<C> {
    async fn list_folders(&self) -> AnyResult<Folders> {
        let feature = self
            .list_folders
            .clone()
            .ok_or(Error::ListFoldersNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::ListFoldersNotAvailableError)?
                    .list_folders()
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> ExpungeFolder for BackendPool<C> {
    async fn expunge_folder(&self, folder: &str) -> AnyResult<()> {
        let folder = folder.to_owned();
        let feature = self
            .expunge_folder
            .clone()
            .ok_or(Error::ExpungeFolderNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::ExpungeFolderNotAvailableError)?
                    .expunge_folder(&folder)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> PurgeFolder for BackendPool<C> {
    async fn purge_folder(&self, folder: &str) -> AnyResult<()> {
        let folder = folder.to_owned();
        let feature = self
            .purge_folder
            .clone()
            .ok_or(Error::PurgeFolderNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::PurgeFolderNotAvailableError)?
                    .purge_folder(&folder)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> DeleteFolder for BackendPool<C> {
    async fn delete_folder(&self, folder: &str) -> AnyResult<()> {
        let folder = folder.to_owned();
        let feature = self
            .delete_folder
            .clone()
            .ok_or(Error::DeleteFolderNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::DeleteFolderNotAvailableError)?
                    .delete_folder(&folder)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> GetEnvelope for BackendPool<C> {
    async fn get_envelope(&self, folder: &str, id: &SingleId) -> AnyResult<Envelope> {
        let folder = folder.to_owned();
        let id = id.clone();
        let feature = self
            .get_envelope
            .clone()
            .ok_or(Error::GetEnvelopeNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::GetEnvelopeNotAvailableError)?
                    .get_envelope(&folder, &id)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> ListEnvelopes for BackendPool<C> {
    async fn list_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<Envelopes> {
        let folder = folder.to_owned();
        let feature = self
            .list_envelopes
            .clone()
            .ok_or(Error::ListEnvelopesNotAvailableError)?;

        self.pool
            .exec(move |ctx| async move {
                feature(&ctx)
                    .ok_or(Error::ListEnvelopesNotAvailableError)?
                    .list_envelopes(&folder, opts)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> ThreadEnvelopes for BackendPool<C> {
    async fn thread_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes> {
        let folder = folder.to_owned();
        let feature = self
            .thread_envelopes
            .clone()
            .ok_or(Error::ThreadEnvelopesNotAvailableError)?;

        self.pool
            .exec(move |ctx| async move {
                feature(&ctx)
                    .ok_or(Error::ThreadEnvelopesNotAvailableError)?
                    .thread_envelopes(&folder, opts)
                    .await
            })
            .await
    }

    async fn thread_envelope(
        &self,
        folder: &str,
        id: SingleId,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes> {
        let folder = folder.to_owned();
        let feature = self
            .thread_envelopes
            .clone()
            .ok_or(Error::ThreadEnvelopesNotAvailableError)?;

        self.pool
            .exec(move |ctx| async move {
                feature(&ctx)
                    .ok_or(Error::ThreadEnvelopesNotAvailableError)?
                    .thread_envelope(&folder, id, opts)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> WatchEnvelopes for BackendPool<C> {
    async fn watch_envelopes(
        &self,
        folder: &str,
        wait_for_shutdown_request: Receiver<()>,
        shutdown: Sender<()>,
    ) -> AnyResult<()> {
        let folder = folder.to_owned();
        let feature = self
            .watch_envelopes
            .clone()
            .ok_or(Error::WatchEnvelopesNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::WatchEnvelopesNotAvailableError)?
                    .watch_envelopes(&folder, wait_for_shutdown_request, shutdown)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> AddFlags for BackendPool<C> {
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        let folder = folder.to_owned();
        let id = id.clone();
        let flags = flags.clone();
        let feature = self
            .add_flags
            .clone()
            .ok_or(Error::AddFlagsNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::AddFlagsNotAvailableError)?
                    .add_flags(&folder, &id, &flags)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> SetFlags for BackendPool<C> {
    async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        let folder = folder.to_owned();
        let id = id.clone();
        let flags = flags.clone();
        let feature = self
            .set_flags
            .clone()
            .ok_or(Error::SetFlagsNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::SetFlagsNotAvailableError)?
                    .set_flags(&folder, &id, &flags)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> RemoveFlags for BackendPool<C> {
    async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        let folder = folder.to_owned();
        let id = id.clone();
        let flags = flags.clone();
        let feature = self
            .remove_flags
            .clone()
            .ok_or(Error::RemoveFlagsNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::RemoveFlagsNotAvailableError)?
                    .remove_flags(&folder, &id, &flags)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> AddMessage for BackendPool<C> {
    async fn add_message_with_flags(
        &self,
        folder: &str,
        msg: &[u8],
        flags: &Flags,
    ) -> AnyResult<SingleId> {
        let folder = folder.to_owned();
        let msg = msg.to_owned();
        let flags = flags.clone();
        let feature = self
            .add_message
            .clone()
            .ok_or(Error::AddMessageNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::AddMessageWithFlagsNotAvailableError)?
                    .add_message_with_flags(&folder, &msg, &flags)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> SendMessage for BackendPool<C> {
    async fn send_message(&self, msg: &[u8]) -> AnyResult<()> {
        let msg = msg.to_owned();
        let feature = self
            .send_message
            .clone()
            .ok_or(Error::SendMessageNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::SendMessageNotAvailableError)?
                    .send_message(&msg)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> PeekMessages for BackendPool<C> {
    async fn peek_messages(&self, folder: &str, id: &Id) -> AnyResult<Messages> {
        let folder = folder.to_owned();
        let id = id.clone();
        let feature = self
            .peek_messages
            .clone()
            .ok_or(Error::PeekMessagesNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::PeekMessagesNotAvailableError)?
                    .peek_messages(&folder, &id)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> GetMessages for BackendPool<C> {
    async fn get_messages(&self, folder: &str, id: &Id) -> AnyResult<Messages> {
        let folder = folder.to_owned();
        let id = id.clone();
        let feature = self
            .get_messages
            .clone()
            .ok_or(Error::GetMessagesNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::GetMessagesNotAvailableError)?
                    .get_messages(&folder, &id)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> CopyMessages for BackendPool<C> {
    async fn copy_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> AnyResult<()> {
        let from_folder = from_folder.to_owned();
        let to_folder = to_folder.to_owned();
        let id = id.clone();
        let feature = self
            .copy_messages
            .clone()
            .ok_or(Error::CopyMessagesNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::CopyMessagesNotAvailableError)?
                    .copy_messages(&from_folder, &to_folder, &id)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> MoveMessages for BackendPool<C> {
    async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> AnyResult<()> {
        let from_folder = from_folder.to_owned();
        let to_folder = to_folder.to_owned();
        let id = id.clone();
        let feature = self
            .move_messages
            .clone()
            .ok_or(Error::MoveMessagesNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::MoveMessagesNotAvailableError)?
                    .move_messages(&from_folder, &to_folder, &id)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<C: BackendContext + 'static> DeleteMessages for BackendPool<C> {
    async fn delete_messages(&self, folder: &str, id: &Id) -> AnyResult<()> {
        let folder = folder.to_owned();
        let id = id.clone();
        let feature = self
            .delete_messages
            .clone()
            .ok_or(Error::DeleteMessagesNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::DeleteMessagesNotAvailableError)?
                    .delete_messages(&folder, &id)
                    .await
            })
            .await
    }
}

#[async_trait]
impl<CB> AsyncTryIntoBackendFeatures<BackendPool<CB::Context>> for BackendBuilder<CB>
where
    CB: BackendContextBuilder + 'static,
{
    async fn try_into_backend(self) -> AnyResult<BackendPool<CB::Context>> {
        let add_folder = self.get_add_folder();
        let list_folders = self.get_list_folders();
        let expunge_folder = self.get_expunge_folder();
        let purge_folder = self.get_purge_folder();
        let delete_folder = self.get_delete_folder();

        let get_envelope = self.get_get_envelope();
        let list_envelopes = self.get_list_envelopes();
        let thread_envelopes = self.get_thread_envelopes();
        let watch_envelopes = self.get_watch_envelopes();

        let add_flags = self.get_add_flags();
        let set_flags = self.get_set_flags();
        let remove_flags = self.get_remove_flags();

        let add_message = self.get_add_message();
        let send_message = self.get_send_message();
        let peek_messages = self.get_peek_messages();
        let get_messages = self.get_get_messages();
        let copy_messages = self.get_copy_messages();
        let move_messages = self.get_move_messages();
        let delete_messages = self.get_delete_messages();

        Ok(BackendPool {
            account_config: self.account_config.clone(),
            pool: ThreadPoolBuilder::new(self.ctx_builder).build().await?,

            add_folder,
            list_folders,
            expunge_folder,
            purge_folder,
            delete_folder,

            get_envelope,
            list_envelopes,
            thread_envelopes,
            watch_envelopes,

            add_flags,
            set_flags,
            remove_flags,

            add_message,
            send_message,
            peek_messages,
            get_messages,
            copy_messages,
            move_messages,
            delete_messages,
        })
    }
}

impl<T> ThreadPoolContext for T where T: BackendContext {}

#[async_trait]
impl<T> ThreadPoolContextBuilder for T
where
    T: BackendContextBuilder,
{
    type Context = T::Context;

    async fn build(self) -> AnyResult<Self::Context> {
        BackendContextBuilder::build(self).await
    }
}
