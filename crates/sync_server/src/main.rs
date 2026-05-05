use std::{
    collections::BTreeMap,
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::Context;
use axum::{
    Json, Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use geometry_core::Point;
use layout_model::{
    ClientMessage, CrdtApplyResult, CrdtOpId, CrdtOperation, Document, LoggedOperation,
    LoroCrdtLog, ServerMessage, ShapeOccurrenceId,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast};
use tracing::{error, info};
use uuid::Uuid;

const PERSISTED_STATE_VERSION: u32 = 1;

#[derive(Clone)]
struct AppState {
    document: Arc<Mutex<Document>>,
    loro_log: Arc<Mutex<LoroCrdtLog>>,
    sequence: Arc<AtomicU64>,
    bus: broadcast::Sender<ServerMessage>,
    cursors: Arc<Mutex<BTreeMap<Uuid, Point>>>,
    selections: Arc<Mutex<BTreeMap<Uuid, Vec<ShapeOccurrenceId>>>>,
    server_actor: Uuid,
    persistence_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedSyncState {
    schema_version: u32,
    document: Document,
    sequence: u64,
    loro_snapshot: Vec<u8>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (bus, _) = broadcast::channel(4096);
    let server_actor = Uuid::new_v4();
    let persistence_path = persistence_path();
    let (document, loro_log, sequence) =
        load_or_create_state(server_actor, persistence_path.as_deref())?;
    let state = AppState {
        document: Arc::new(Mutex::new(document)),
        loro_log: Arc::new(Mutex::new(loro_log)),
        sequence: Arc::new(AtomicU64::new(sequence)),
        bus,
        cursors: Arc::new(Mutex::new(BTreeMap::new())),
        selections: Arc::new(Mutex::new(BTreeMap::new())),
        server_actor,
        persistence_path,
    };
    let app = Router::new()
        .route("/health", get(health))
        .route("/snapshot", get(snapshot))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr: SocketAddr = std::env::var("FABRICAD_SYNC_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:4141".to_string())
        .parse()
        .context("FABRICAD_SYNC_ADDR must be host:port")?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("fabricad sync server listening on ws://{addr}/ws");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn snapshot(State(state): State<AppState>) -> Json<ServerMessage> {
    let document = state.document.lock().await.clone();
    let cursors = state.cursors.lock().await.clone();
    let selections = state.selections.lock().await.clone();
    let loro_snapshot = state
        .loro_log
        .lock()
        .await
        .export_snapshot()
        .unwrap_or_default();
    Json(ServerMessage::Snapshot {
        document,
        cursors,
        selections,
        loro_snapshot,
    })
}

async fn ws_handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(state, socket))
}

async fn handle_socket(state: AppState, socket: WebSocket) {
    let mut rx = state.bus.subscribe();
    let (mut sender, mut receiver) = socket.split();
    let initial = {
        let document = state.document.lock().await.clone();
        let cursors = state.cursors.lock().await.clone();
        let selections = state.selections.lock().await.clone();
        let loro_snapshot = state
            .loro_log
            .lock()
            .await
            .export_snapshot()
            .unwrap_or_default();
        ServerMessage::Snapshot {
            document,
            cursors,
            selections,
            loro_snapshot,
        }
    };
    if send_json(&mut sender, &initial).await.is_err() {
        return;
    }

    let mut joined_user = None;
    loop {
        tokio::select! {
            broadcast = rx.recv() => {
                match broadcast {
                    Ok(message) => {
                        if send_json(&mut sender, &message).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        let _ = send_json(&mut sender, &ServerMessage::Error {
                            message: format!("client lagged by {skipped} collaboration messages"),
                        }).await;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming = receiver.next() => {
                let Some(incoming) = incoming else {
                    break;
                };
                match incoming {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(message) => {
                                if let Some(user) = handle_client_message(&state, message).await {
                                    joined_user = Some(user);
                                }
                            }
                            Err(err) => {
                                let _ = send_json(&mut sender, &ServerMessage::Error {
                                    message: format!("invalid collaboration message: {err}"),
                                }).await;
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(Message::Ping(payload)) => {
                        if sender.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(err) => {
                        error!("websocket error: {err}");
                        break;
                    }
                }
            }
        }
    }

    if let Some(user) = joined_user {
        state.cursors.lock().await.remove(&user);
        state.selections.lock().await.remove(&user);
        let _ = state.bus.send(ServerMessage::UserLeft { user });
    }
}

async fn handle_client_message(state: &AppState, message: ClientMessage) -> Option<Uuid> {
    match message {
        ClientMessage::Join { user } => {
            let _ = state.bus.send(ServerMessage::UserJoined { user });
            Some(user)
        }
        ClientMessage::Operation { user, operation } => {
            let sequence = state.sequence.fetch_add(1, Ordering::SeqCst);
            let logged = LoggedOperation {
                sequence,
                user,
                operation: operation.clone(),
            };
            {
                let mut document = state.document.lock().await;
                document.apply_operation(logged.clone());
            }
            let loro_operation = CrdtOperation {
                id: CrdtOpId {
                    actor: state.server_actor,
                    counter: sequence,
                },
                deps: Vec::new(),
                operation,
            };
            if let Err(err) = state
                .loro_log
                .lock()
                .await
                .append_operation(user, loro_operation)
            {
                let _ = state.bus.send(ServerMessage::Error {
                    message: format!("failed to mirror operation into Loro: {err}"),
                });
            }
            let _ = state
                .bus
                .send(ServerMessage::Operation { operation: logged });
            persist_state(state).await;
            None
        }
        ClientMessage::CrdtOperation { operation } => {
            let result = state
                .document
                .lock()
                .await
                .apply_crdt_operation(operation.clone());
            if result == CrdtApplyResult::Applied {
                if let Err(err) = state
                    .loro_log
                    .lock()
                    .await
                    .append_operation(operation.id.actor, operation.clone())
                {
                    let _ = state.bus.send(ServerMessage::Error {
                        message: format!("failed to mirror CRDT operation into Loro: {err}"),
                    });
                }
                let _ = state.bus.send(ServerMessage::CrdtOperation { operation });
                persist_state(state).await;
            }
            None
        }
        ClientMessage::LoroUpdate { update } => {
            let operations = match state.loro_log.lock().await.import_update(&update) {
                Ok(operations) => operations,
                Err(err) => {
                    let _ = state.bus.send(ServerMessage::Error {
                        message: format!("failed to import Loro update: {err}"),
                    });
                    return None;
                }
            };
            if !operations.is_empty() {
                {
                    let mut document = state.document.lock().await;
                    for operation in operations {
                        document.apply_crdt_operation(operation);
                    }
                }
                let _ = state.bus.send(ServerMessage::LoroUpdate { update });
                persist_state(state).await;
            }
            None
        }
        ClientMessage::Cursor { user, position } => {
            state.cursors.lock().await.insert(user, position);
            let _ = state.bus.send(ServerMessage::Cursor { user, position });
            None
        }
        ClientMessage::Selection { user, selection } => {
            if selection.is_empty() {
                state.selections.lock().await.remove(&user);
            } else {
                state
                    .selections
                    .lock()
                    .await
                    .insert(user, selection.clone());
            }
            let _ = state.bus.send(ServerMessage::Selection { user, selection });
            None
        }
        ClientMessage::RequestSnapshot => {
            let document = state.document.lock().await.clone();
            let cursors = state.cursors.lock().await.clone();
            let selections = state.selections.lock().await.clone();
            let loro_snapshot = state
                .loro_log
                .lock()
                .await
                .export_snapshot()
                .unwrap_or_default();
            let _ = state.bus.send(ServerMessage::Snapshot {
                document,
                cursors,
                selections,
                loro_snapshot,
            });
            None
        }
    }
}

async fn send_json(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    message: &ServerMessage,
) -> Result<(), axum::Error> {
    let text = serde_json::to_string(message).unwrap_or_else(|err| {
        format!(r#"{{"type":"error","message":"failed to serialize server message: {err}"}}"#)
    });
    sender.send(Message::Text(text.into())).await
}

fn persistence_path() -> Option<PathBuf> {
    match std::env::var("FABRICAD_SYNC_STATE") {
        Ok(value) if value.trim().is_empty() || value == "off" || value == "none" => None,
        Ok(value) => Some(PathBuf::from(value)),
        Err(_) => Some(PathBuf::from("target/fabricad-sync/state.json")),
    }
}

fn load_or_create_state(
    server_actor: Uuid,
    path: Option<&Path>,
) -> anyhow::Result<(Document, LoroCrdtLog, u64)> {
    if let Some(path) = path {
        if path.exists() {
            let bytes = fs::read(path).with_context(|| {
                format!("failed to read persisted sync state {}", path.display())
            })?;
            if bytes.is_empty() {
                info!(
                    "persisted sync state {} is empty; starting from demo document",
                    path.display()
                );
            } else {
                let persisted: PersistedSyncState =
                    serde_json::from_slice(&bytes).with_context(|| {
                        format!("failed to parse persisted sync state {}", path.display())
                    })?;
                let mut document = persisted.document;
                let loro_log = LoroCrdtLog::from_snapshot(server_actor, &persisted.loro_snapshot)
                    .context("failed to import persisted Loro snapshot")?;
                if !persisted.loro_snapshot.is_empty() {
                    loro_log
                        .materialize_objects_into_document(&mut document)
                        .context("failed to materialize persisted Loro object store")?;
                }
                info!(
                    "loaded sync state from {} at sequence {}",
                    path.display(),
                    persisted.sequence
                );
                return Ok((document, loro_log, persisted.sequence.max(1)));
            }
        }
    }

    let document = Document::demo();
    let mut loro_log = LoroCrdtLog::new(server_actor)?;
    loro_log.seed_document_objects(&document)?;
    Ok((document, loro_log, 1))
}

async fn persist_state(state: &AppState) {
    let Some(path) = state.persistence_path.as_deref() else {
        return;
    };
    let document = state.document.lock().await.clone();
    let loro_snapshot = match state.loro_log.lock().await.export_snapshot() {
        Ok(snapshot) => snapshot,
        Err(err) => {
            error!("failed to export Loro snapshot for persistence: {err}");
            return;
        }
    };
    let persisted = PersistedSyncState {
        schema_version: PERSISTED_STATE_VERSION,
        document,
        sequence: state.sequence.load(Ordering::SeqCst),
        loro_snapshot,
    };
    if let Err(err) = write_persisted_state(path, &persisted) {
        error!("failed to persist sync state to {}: {err}", path.display());
    }
}

fn write_persisted_state(path: &Path, state: &PersistedSyncState) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create state directory {}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(state)?;
    fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use geometry_core::Rect;
    use layout_model::{LayerId, Operation, ProcessLayer, Shape, ShapeId, ShapeKind};
    use tokio_tungstenite::{
        MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message as ClientWsMessage,
    };

    type TestSocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

    async fn send_client_message(socket: &mut TestSocket, message: &ClientMessage) {
        socket
            .send(ClientWsMessage::Text(
                serde_json::to_string(message).unwrap().into(),
            ))
            .await
            .unwrap();
    }

    async fn read_server_message(socket: &mut TestSocket) -> ServerMessage {
        loop {
            match socket.next().await.unwrap().unwrap() {
                ClientWsMessage::Text(text) => {
                    return serde_json::from_str(&text)
                        .unwrap_or_else(|err| panic!("invalid server message {text}: {err}"));
                }
                ClientWsMessage::Ping(payload) => {
                    socket.send(ClientWsMessage::Pong(payload)).await.unwrap();
                }
                _ => {}
            }
        }
    }

    async fn read_until(
        socket: &mut TestSocket,
        mut predicate: impl FnMut(&ServerMessage) -> bool,
    ) -> ServerMessage {
        loop {
            let message = read_server_message(socket).await;
            if predicate(&message) {
                return message;
            }
        }
    }

    #[test]
    fn persisted_startup_materializes_loro_object_store() {
        let server_actor = Uuid::from_u128(301);
        let client_actor = Uuid::from_u128(302);
        let document = Document::new("persisted fallback");
        let layer = document
            .layer_by_process(layout_model::ProcessLayer::Metal1)
            .unwrap();
        let shape = Shape {
            id: ShapeId(99),
            layer,
            net: None,
            kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(10, 20), 100, 80)),
            name: None,
        };
        let mut log = LoroCrdtLog::new(client_actor).unwrap();
        log.append_operation(
            client_actor,
            CrdtOperation {
                id: CrdtOpId {
                    actor: client_actor,
                    counter: 1,
                },
                deps: Vec::new(),
                operation: Operation::AddShape {
                    shape: shape.clone(),
                },
            },
        )
        .unwrap();
        let persisted = PersistedSyncState {
            schema_version: PERSISTED_STATE_VERSION,
            document: document.clone(),
            sequence: 7,
            loro_snapshot: log.export_snapshot().unwrap(),
        };
        let path = std::env::temp_dir().join(format!(
            "fabricad-sync-state-{}.json",
            Uuid::new_v4().simple()
        ));
        write_persisted_state(&path, &persisted).unwrap();

        let (loaded, _loaded_log, sequence) =
            load_or_create_state(server_actor, Some(path.as_path())).unwrap();
        let _ = fs::remove_file(path);

        assert_eq!(sequence, 7);
        assert_eq!(
            loaded.shapes.get(&shape.id).unwrap().kind.bounds(),
            shape.kind.bounds()
        );
        assert_eq!(loaded.layers.get(&LayerId(layer.0)).unwrap().id, layer);
    }

    #[tokio::test]
    async fn websocket_clients_share_loro_edits_presence_and_reconnect_snapshot() {
        let (bus, _) = broadcast::channel(256);
        let server_actor = Uuid::from_u128(401);
        let document = Document::new("websocket collaboration smoke");
        let metal1 = document.layer_by_process(ProcessLayer::Metal1).unwrap();
        let mut loro_log = LoroCrdtLog::new(server_actor).unwrap();
        loro_log.seed_document_objects(&document).unwrap();
        let state = AppState {
            document: Arc::new(Mutex::new(document.clone())),
            loro_log: Arc::new(Mutex::new(loro_log)),
            sequence: Arc::new(AtomicU64::new(1)),
            bus,
            cursors: Arc::new(Mutex::new(BTreeMap::new())),
            selections: Arc::new(Mutex::new(BTreeMap::new())),
            server_actor,
            persistence_path: None,
        };
        let app = Router::new()
            .route("/health", get(health))
            .route("/snapshot", get(snapshot))
            .route("/ws", get(ws_handler))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let url = format!("ws://{addr}/ws");

        let user_a = Uuid::from_u128(411);
        let user_b = Uuid::from_u128(412);
        let user_c = Uuid::from_u128(413);
        let user_d = Uuid::from_u128(414);
        let (mut client_a, _) = connect_async(&url).await.unwrap();
        let (mut client_b, _) = connect_async(&url).await.unwrap();
        let (mut client_c, _) = connect_async(&url).await.unwrap();
        let (mut client_d, _) = connect_async(&url).await.unwrap();
        assert!(matches!(
            read_server_message(&mut client_a).await,
            ServerMessage::Snapshot { .. }
        ));
        assert!(matches!(
            read_server_message(&mut client_b).await,
            ServerMessage::Snapshot { .. }
        ));
        assert!(matches!(
            read_server_message(&mut client_c).await,
            ServerMessage::Snapshot { .. }
        ));
        assert!(matches!(
            read_server_message(&mut client_d).await,
            ServerMessage::Snapshot { .. }
        ));

        for (socket, user) in [
            (&mut client_a, user_a),
            (&mut client_b, user_b),
            (&mut client_c, user_c),
            (&mut client_d, user_d),
        ] {
            send_client_message(socket, &ClientMessage::Join { user }).await;
        }

        let shape = Shape {
            id: ShapeId(700),
            layer: metal1,
            net: None,
            kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(100, 200), 300, 120)),
            name: Some("shared_m1".to_string()),
        };
        let mut client_log = LoroCrdtLog::new(user_a).unwrap();
        let add_update = client_log
            .append_operation(
                user_a,
                CrdtOperation {
                    id: CrdtOpId {
                        actor: user_a,
                        counter: 1,
                    },
                    deps: Vec::new(),
                    operation: Operation::AddShape {
                        shape: shape.clone(),
                    },
                },
            )
            .unwrap();
        send_client_message(
            &mut client_a,
            &ClientMessage::LoroUpdate { update: add_update },
        )
        .await;
        for socket in [&mut client_b, &mut client_c, &mut client_d] {
            read_until(socket, |message| {
                matches!(message, ServerMessage::LoroUpdate { .. })
            })
            .await;
        }

        let selection = vec![ShapeOccurrenceId::top_level(shape.id)];
        send_client_message(
            &mut client_b,
            &ClientMessage::Selection {
                user: user_b,
                selection: selection.clone(),
            },
        )
        .await;
        send_client_message(
            &mut client_c,
            &ClientMessage::Cursor {
                user: user_c,
                position: Point::new(500, 600),
            },
        )
        .await;
        assert!(matches!(
            read_until(&mut client_a, |message| {
                matches!(
                    message,
                    ServerMessage::Selection {
                        user,
                        selection: received,
                    } if *user == user_b && *received == selection
                )
            })
            .await,
            ServerMessage::Selection { .. }
        ));
        assert!(matches!(
            read_until(&mut client_a, |message| {
                matches!(
                    message,
                    ServerMessage::Cursor { user, position }
                        if *user == user_c && *position == Point::new(500, 600)
                )
            })
            .await,
            ServerMessage::Cursor { .. }
        ));

        drop(client_d);
        let move_update = client_log
            .append_operation(
                user_a,
                CrdtOperation {
                    id: CrdtOpId {
                        actor: user_a,
                        counter: 2,
                    },
                    deps: vec![CrdtOpId {
                        actor: user_a,
                        counter: 1,
                    }],
                    operation: Operation::MoveShape {
                        id: shape.id,
                        delta: geometry_core::Vector::new(80, -40),
                    },
                },
            )
            .unwrap();
        send_client_message(
            &mut client_a,
            &ClientMessage::LoroUpdate {
                update: move_update,
            },
        )
        .await;
        read_until(&mut client_b, |message| {
            matches!(message, ServerMessage::LoroUpdate { .. })
        })
        .await;

        let (mut reconnected_d, _) = connect_async(&url).await.unwrap();
        let snapshot = read_server_message(&mut reconnected_d).await;
        let ServerMessage::Snapshot {
            document,
            cursors,
            selections,
            loro_snapshot,
        } = snapshot
        else {
            panic!("expected reconnect snapshot");
        };
        assert!(!loro_snapshot.is_empty());
        assert_eq!(
            document.shapes.get(&shape.id).unwrap().kind.bounds(),
            Rect::from_min_size(Point::new(180, 160), 300, 120)
        );
        assert_eq!(cursors.get(&user_c), Some(&Point::new(500, 600)));
        assert_eq!(selections.get(&user_b), Some(&selection));

        server.abort();
    }
}
