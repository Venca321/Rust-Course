use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::Html,
    routing::{delete, get},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    user: String,
    content: String,
}

#[derive(Default, Clone)]
struct AppState {
    messages: Arc<RwLock<Vec<Message>>>,
}

#[tokio::main]
async fn main() {
    let app_state = AppState::default();

    let app = Router::new()
        .route("/", get(show_messages))
        .route("/filter", get(filter_messages))
        .route("/delete_user/:user", delete(delete_user))
        .layer(Extension(app_state));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

async fn show_messages(Extension(state): Extension<AppState>) -> Html<String> {
    let messages = state.messages.read().await;
    let mut html = String::from("<h1>Messages</h1><ul>");
    for message in messages.iter() {
        html.push_str(&format!("<li>{}: {}</li>", message.user, message.content));
    }
    html.push_str("</ul>");
    Html(html)
}

async fn filter_messages(
    Extension(state): Extension<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Html<String> {
    let user = params.get("user").cloned().unwrap_or_default();
    let messages = state.messages.read().await;
    let mut html = String::from("<h1>Filtered Messages</h1><ul>");
    for message in messages.iter().filter(|m| m.user == user) {
        html.push_str(&format!("<li>{}: {}</li>", message.user, message.content));
    }
    html.push_str("</ul>");
    Html(html)
}

async fn delete_user(
    Extension(state): Extension<AppState>,
    Path(user): Path<String>,
) -> StatusCode {
    let mut messages = state.messages.write().await;
    messages.retain(|message| message.user != user);
    StatusCode::NO_CONTENT
}
