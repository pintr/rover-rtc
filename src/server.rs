use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Mutex, mpsc},
};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use tracing::{error, info, warn};

type PeerMap = Arc<Mutex<HashMap<String, mpsc::Sender<String>>>>;

// A simple signaling server that relays messages between two peers in a "room".
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    info!("Signaling server listening on ws://{}", addr);

    // For this prototype, we'll use a single room named "rover-room".
    let peers = PeerMap::new(Mutex::new(HashMap::new()));

    while let Ok((stream, peer_addr)) = listener.accept().await {
        info!("Peer connected: {}", peer_addr);
        let peers_clone = Arc::clone(&peers);
        tokio::spawn(handle_connection(stream, peers_clone));
    }

    Ok(())
}

async fn handle_connection(stream: TcpStream, peers: PeerMap) {
    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake error: {}", e);
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // For simplicity, the first message from a client is its ID.
    let peer_id = match ws_receiver.next().await {
        Some(Ok(Message::Text(id))) => id,
        _ => {
            warn!("Peer did not send an ID. Disconnecting.");
            return;
        }
    };
    info!("Peer '{}' registered.", peer_id);

    let (mpsc_tx, mut mpsc_rx) = mpsc::channel::<String>(100);
    peers.lock().await.insert(peer_id.clone(), mpsc_tx);

    // Task to forward messages from the MPSC channel to this peer's WebSocket.
    let mut send_task = tokio::spawn(async move {
        while let Some(msg_str) = mpsc_rx.recv().await {
            if ws_sender.send(Message::Text(msg_str)).await.is_err() {
                warn!("Failed to send message to peer '{}'; it may have disconnected.", peer_id);
                break;
            }
        }
    });

    // Task to receive messages from this peer's WebSocket and broadcast them.
    let peers_clone = Arc::clone(&peers);
    let peer_id_clone = peer_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if let Message::Text(text_msg) = msg {
                let peers_guard = peers_clone.lock().await;
                // Broadcast to all other peers in the room.
                for (id, tx) in peers_guard.iter() {
                    if *id != peer_id_clone {
                        if tx.send(text_msg.clone()).await.is_err() {
                            warn!("Failed to relay message to peer '{}'; channel closed.", id);
                        }
                    }
                }
            }
        }
    });

    // Wait for one of the tasks to finish (which means the client disconnected).
    tokio::select! {
        _ = &mut send_task => (),
        _ = &mut recv_task => (),
    };

    info!("Peer '{}' disconnected.", peer_id);
    peers.lock().await.remove(&peer_id);
}

