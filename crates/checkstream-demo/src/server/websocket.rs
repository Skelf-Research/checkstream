use crate::models::DemoEvent;
use crate::state::DemoAppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::time::interval;

/// WebSocket handler for real-time event streaming
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<DemoAppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: DemoAppState) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to event bus
    let mut events = state.event_bus.subscribe();

    // Send initial metrics snapshot
    let initial = DemoEvent::MetricsUpdate(state.metrics.snapshot());
    if let Ok(msg) = serde_json::to_string(&initial) {
        let _ = sender.send(Message::Text(msg)).await;
    }

    // Periodic metrics update task
    let metrics = state.metrics.clone();
    let event_bus = state.event_bus.clone();
    let metrics_task = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;
            let snapshot = metrics.snapshot();
            event_bus.publish(DemoEvent::MetricsUpdate(snapshot));
        }
    });

    // Event forwarding task
    let send_task = tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            match serde_json::to_string(&event) {
                Ok(msg) => {
                    if sender.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to serialize event: {}", e);
                }
            }
        }
    });

    // Receive task (handle client messages/pings)
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Ping(data) => {
                    // Pong is handled automatically by axum
                    tracing::trace!("Received ping: {:?}", data);
                }
                Message::Text(text) => {
                    // Handle client commands if needed
                    tracing::trace!("Received message: {}", text);
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {
            tracing::debug!("Send task completed");
        }
        _ = recv_task => {
            tracing::debug!("Receive task completed");
        }
    }

    metrics_task.abort();
}
