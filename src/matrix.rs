use std::io;

use futures::stream::StreamExt;
use matrix_sdk::config::SyncSettings;
use matrix_sdk::matrix_auth::{MatrixSession, MatrixSessionTokens};
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use matrix_sdk::{Client, SessionMeta};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;
use tracing::instrument;

use crate::config::MatrixConfig;

#[instrument(skip(room_rx))]
pub async fn matrix_init(
    config: &MatrixConfig,
    mut room_rx: UnboundedReceiver<String>,
) -> anyhow::Result<Vec<JoinHandle<io::Result<()>>>> {
    let client = Client::new(config.homeserver.clone()).await?;

    let session = MatrixSession {
        meta: SessionMeta {
            user_id: config.user_id.to_owned(),
            device_id: config.device_id.to_owned(),
        },
        tokens: MatrixSessionTokens {
            access_token: config.access_token.to_owned(),
            refresh_token: None,
        },
    };

    client.restore_session(session).await?;

    tracing::debug!("Doing first sync");
    if let Err(err) = client.sync_once(SyncSettings::default()).await {
        tracing::error!("Client::sync_once() error: {:?}", err);
    }
    tracing::debug!("First sync done");

    let mut handles = Vec::new();
    if let Ok(resp) = client.join_room_by_id(&config.room_id).await {
        if let Some(room) = client.get_room(resp.room_id()) {
            let handle = tokio::spawn(async move {
                while let Some(line) = room_rx.recv().await {
                    tracing::info!("matrix tx: ^{line}$");
                    let content = RoomMessageEventContent::notice_plain(line);
                    let resp = room.send(content).await;
                    tracing::debug!("Room message send response: {resp:?}");
                }
                Ok(())
            });
            handles.push(handle);
        }
    }

    let handle = tokio::spawn(async move {
        let mut sync_stream = Box::pin(client.sync_stream(SyncSettings::default()).await);
        while let Some(res) = sync_stream.next().await {
            match res {
                Ok(_) => (),
                Err(err) => {
                    tracing::error!("sync_stream returned error: {err}");
                    return Err(io::Error::new(io::ErrorKind::Interrupted, err));
                }
            }
        }
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "sync_stream died",
        ))
    });
    handles.push(handle);

    Ok(handles)
}
