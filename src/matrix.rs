use std::io;

use matrix_sdk::config::SyncSettings;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use matrix_sdk::ruma::TransactionId;
use matrix_sdk::{Client, Session};
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

    let session = Session {
        access_token: config.access_token.to_owned(),
        refresh_token: None,
        user_id: config.user_id.to_owned(),
        device_id: config.device_id.to_owned(),
    };

    client.restore_login(session).await?;

    tracing::debug!("Doing first sync");
    if let Err(err) = client.sync_once(SyncSettings::default()).await {
        tracing::error!("Client::sync_once() error: {:?}", err);
    }
    tracing::debug!("First sync done");

    let mut handles = Vec::new();
    if let Ok(resp) = client.join_room_by_id(&config.room_id).await {
        if let Some(room) = client.get_joined_room(&resp.room_id) {
            let handle = tokio::spawn(async move {
                while let Some(line) = room_rx.recv().await {
                    tracing::debug!("matrix tx: ^{line}$");
                    let content = RoomMessageEventContent::notice_plain(line);
                    let txn_id = TransactionId::new();
                    let resp = room.send(content, Some(&txn_id)).await;
                    tracing::debug!("Room message send response: {resp:?}");
                }
                Ok(())
            });
            handles.push(handle);
        }
    }

    let handle = tokio::spawn(async move {
        client
            .sync(SyncSettings::default())
            .await
            .map_err(|err| io::Error::new(io::ErrorKind::Interrupted, err))
    });
    handles.push(handle);

    Ok(handles)
}
