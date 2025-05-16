use std::{path::PathBuf, sync::Arc};

use anyhow::{anyhow, bail};
use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs, ChatCompletionRequestUserMessageArgs
};
use axum::{
    Router,
    body::Bytes,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
    routing::{any, get},
};
use chrono::Utc;
use clap::Parser;
use maud::{DOCTYPE, Markup, html};
use rmcp::{model::{RawContent, RawTextContent}, serde_json};
use rullm::{args::Args, env::Env};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::{debug, info, instrument, level_filters::LevelFilter, warn};
use tracing_subscriber::{Layer as _, layer::SubscriberExt as _, util::SubscriberInitExt as _};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(LevelFilter::DEBUG))
        .init();

    let args = Args::parse();
    let env = Arc::new(Env::build(args).await?);

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");

    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir))
        .route("/", get(get_index))
        .route("/ws", any(ws_handler))
        .with_state(env.clone())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    axum::serve(listener, app).await?;

    Ok(())
}

#[instrument]
async fn get_index() -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { "Chat App" }
                script src="https://unpkg.com/htmx.org@1.9.12" {}
                script src="https://unpkg.com/htmx.org@1.9.12/dist/ext/ws.js" {}
                script src="https://unpkg.com/hyperscript.org@0.9.14" {}
                link href="./css/style.css" rel="stylesheet";
            }
            body class="font-sans h-screen overflow-hidden flex flex-col bg-gray-200" {
                div class="max-w-3xl mx-auto h-full w-full" {
                    div class="flex flex-col h-full" hx-ext="ws" ws-connect="/ws" {
                        // Placeholder, will be filled by websockets
                        div class="flex-grow overflow-y-auto p-5 flex flex-col space-y-4" #chat_room {}
                        form class="input-area p-4 border-t border-gray-300 bg-gray-100 flex items-center" #form ws-send _="on submit target.reset()" {
                            textarea class="flex-grow p-2 border border-gray-300 rounded-xl mr-2 resize-none text-base leading-tight min-h-[40px] max-h-[150px] overflow-y-auto focus:outline-none focus:border-blue-500" placeholder="How are you" name="chat_message" {}
                            // input name="chat_message";
                            button type="submit" class="px-5 py-2 bg-blue-500 text-white border-none rounded-xl cursor-pointer text-base flex-shrink-0 hover:bg-blue-600 focus:outline-none" { "Send" }
                        }
                    }
                }
            }
        }
    }
}

#[instrument(skip(ws, env))]
async fn ws_handler(ws: WebSocketUpgrade, State(env): State<Arc<Env>>) -> impl IntoResponse {
    info!("Upgrading websocket");
    ws.on_upgrade(move |socket| handle_socket(socket, env))
}

#[derive(Debug, Serialize, Deserialize)]
struct HtmxMessage {
    chat_message: String,
}


struct HtmxChat {
    socket: WebSocket,
    messages: Vec<ChatCompletionRequestMessage>,
    env: Arc<Env>,
}

impl HtmxChat {
    fn new(socket: WebSocket, env: Arc<Env>) -> Self {
        let mut messages = Vec::new();
        let system_prompt = format!(
            "You are a helpful assistant. You know that today is {}",
            Utc::now().date_naive().format("%Y-%m-%d").to_string()
        );
        messages.push(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_prompt)
                .build().unwrap()
                .into());
        HtmxChat{ socket, messages, env }
    }

    async fn recv(&mut self) -> anyhow::Result<Option<HtmxMessage>> {
        use ProcessError::*;
        match self.socket.recv().await {
            None => Ok(None),
            Some(Err(err)) => Err(anyhow!(err)),
            Some(Ok(Message::Text(data))) => {
                let json = serde_json::from_str::<HtmxMessage>(data.as_str())?;
                Ok(Some(json))
            }
            Some(Ok(msg)) => bail!(UnknownMessage(msg)),
        }
    }

    async fn send(&mut self, msg: Markup) -> anyhow::Result<()> {
        let out_message = html! { div #chat_room class="flex-grow overflow-y-auto p-5 flex flex-col space-y-4" hx-swap-oob="beforeend" { (msg) } };
        self.socket
            .send(Message::Text(out_message.into_string().into()))
        .await?;
        Ok(())
    }

    async fn send_user(&mut self, msg: &str) -> anyhow::Result<()> {
        self.messages.push(ChatCompletionRequestUserMessageArgs::default().content(msg).build()?.into());
        let markup = html! { div class="user-message bg-blue-500 text-white self-end rounded-xl rounded-br-none p-3 max-w-3/4 break-words" { (msg) } };
        self.send(markup).await?;
        Ok(())
    }

    async fn send_assistant(&mut self, msg: &str) -> anyhow::Result<()> {
        self.messages.push(ChatCompletionRequestAssistantMessageArgs::default().content(msg).build()?.into());
        let markup = html! { div #assistant class="assistant-message bg-gray-200 text-black self-start rounded-xl rounded-bl-none p-3 max-w-3/4 break-words" { (msg)} };
        self.send(markup).await?;
        Ok(())
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        loop {
            match self.recv().await? {
                Some(msg) => {
                    self.process_message(&msg.chat_message).await?;
                }
                None => return Ok(())
            }
        }
    }

    async fn process_message(&mut self, msg: &str) -> anyhow::Result<()> {
        self.send_user(&msg).await?;

        loop {
            let response = self.env.openai_client.chat(&self.messages).await?;

            let mut text_responses: Vec<String> = vec![];
            let mut tool_calls: Vec<ChatCompletionMessageToolCall> = vec![];
            for message in response.choices.into_iter().map(|x| x.message) {
                if let Some(content) = message.content {
                    text_responses.push(content);
                }
                if let Some(tools) = message.tool_calls {
                    tool_calls.extend(tools);
                }
            }
            let assistant_response = text_responses.join("\n");

            if tool_calls.is_empty() {
                self.send_assistant(&assistant_response).await?;

                return Ok(())
            }
            else {
                self.process_function_calls(&assistant_response, &tool_calls).await?;
            }

        }
    }

    async fn send_tool_calls(&mut self, calls: Vec<ChatCompletionRequestMessage>) -> anyhow::Result<()> {
        Ok(self.messages.extend(calls))
    }

    async fn process_function_calls(
        &mut self,
        assistant_response: &str,
        tool_calls: &[ChatCompletionMessageToolCall],
    ) -> anyhow::Result<()> {
        let mut messages = vec![];
        messages.push(ChatCompletionRequestAssistantMessageArgs::default().content(assistant_response).tool_calls(Vec::from(tool_calls)).build()?.into());
        for call in Vec::from(tool_calls) {
            debug!("Calling {}", call.function.name);
            let id = call.id;
            let response = self.env.mcp.call_tool(&call.function).await?;
            let mut text_response = String::new();
            for raw in response.content.into_iter().map(|x| x.raw) {
                match raw {
                    RawContent::Text(RawTextContent { text }) => text_response.push_str(&text),
                    x => bail!("Unknown response: {:?}", x),
                }
            }
            messages.push(ChatCompletionRequestToolMessageArgs::default().content(text_response).tool_call_id(id).build()?.into());
        }
        self.send_tool_calls(messages).await?;
        Ok(())
    }
}

#[instrument(skip(socket, env))]
async fn handle_socket(mut socket: WebSocket, env: Arc<Env>) {
    if socket
        .send(Message::Ping(Bytes::from_static(&[1, 2, 3])))
        .await
        .is_ok()
    {
        info!("Pinged");
    } else {
        warn!("Ping failed, exiting");
        return;
    }

    if let Some(Ok(Message::Pong(_))) = socket.recv().await {
        let mut chatter = HtmxChat::new(socket, env);

        // Ping success
        // Start the main loop
        chatter.run().await.expect("Failed");
    }
}

#[derive(Debug, Error)]
enum ProcessError {
    #[error("Unknown message type: {0:?}")]
    UnknownMessage(Message),
}


