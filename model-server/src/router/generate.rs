use crate::{
    api_types::{GenerateRequest, GenerateResponse},
    state::AppState,
};

use axum::{extract::State, http::StatusCode, Json};

#[axum::debug_handler]
pub async fn generate(
    State(app_state): State<AppState>,
    Json(params): Json<GenerateRequest>,
) -> Result<Json<GenerateResponse>, StatusCode> {
    let model = app_state.model;
    let completion = {
        let mut model = model.model.lock().await;
        model.generate(&params.prompt)
    };

    let res = GenerateResponse {
        model_id: params.model_id.clone(),
        completion,
    };

    Ok(Json(res))
}

// New websocket
// pub async fn generate_ws(
//     ws: WebSocketUpgrade,
//     State(app_state): State<AppState>,
// ) -> impl IntoResponse {
//     ws.on_upgrade(move |socket| drive_ws(socket, Arc::clone(&app_state.model)))
// }

// async fn drive_ws(mut ws: WebSocket, model: Arc<ManagedModel>) {
//     if let Some(msg) = ws.recv().await {
//         if let Ok(msg) = msg {
//             handle_message(ws, msg, model).await
//         }
//     }
// }

// async fn handle_message(mut ws: WebSocket, msg: Message, model: Arc<ManagedModel>) {
//     if let Ok(msg) = msg.into_text() {
//         let generate_request = serde_json::from_str::<GenerateRequest>(&msg)
//             .context("parsing JSON from user message to GenerateRequest");
//         if generate_request.is_err() {
//             error!("failed to parse GenerateRequest from stream");
//             return;
//         }

//         let generate_request = generate_request.unwrap();

//         stream_tokens(ws, generate_request, model).await;
//     } else {
//         error!("failed to parse ws message as text");
//     }
// }

// async fn stream_tokens(
//     mut ws: WebSocket,
//     generate_request: GenerateRequest,
//     model: Arc<ManagedModel>,
// ) {
//     let (sender, mut receiver) = tokio::sync::mpsc::channel(0);
//     // Need to await in a background task or something...
//     let model = Arc::clone(&model);
//     tokio::spawn(async {
//         let mut model = model.model.lock().await;
//         model
//             .generate_stream(&generate_request.clone().prompt, sender)
//             .await;
//     });

//     while let Some(msg) = receiver.recv().await {
//         match msg {
//             llamacpp::StreamMessage::Done => {
//                 ws.close().await.context("failed to close ws").unwrap();
//                 return;
//             }
//             llamacpp::StreamMessage::NextToken(token) => ws
//                 .send(Message::Text(token))
//                 .await
//                 .context("failed to send next token to ws")
//                 .unwrap(),
//         }
//     }
// }
