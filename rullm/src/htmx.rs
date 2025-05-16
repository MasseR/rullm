use std::sync::Arc;

use anyhow::{anyhow, bail};
use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessageArgs,
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestToolMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use maud::{Markup, PreEscaped, html};
use rmcp::{
    model::{RawContent, RawTextContent},
    serde_json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

use crate::env::Env;

#[derive(Debug, Error)]
enum ProcessError {
    #[error("Unknown message type: {0:?}")]
    UnknownMessage(Message),
}

#[derive(Debug, Serialize, Deserialize)]
struct HtmxMessage {
    chat_message: String,
}

pub struct HtmxChat {
    socket: WebSocket,
    messages: Vec<ChatCompletionRequestMessage>,
    env: Arc<Env>,
}

impl HtmxChat {
    pub fn new(socket: WebSocket, env: Arc<Env>) -> Self {
        let mut messages = Vec::new();
        let system_prompt = format!(
            "You are a helpful assistant. You know that today is {}",
            Utc::now().date_naive().format("%Y-%m-%d").to_string()
        );
        messages.push(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_prompt)
                .build()
                .unwrap()
                .into(),
        );
        HtmxChat {
            socket,
            messages,
            env,
        }
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
        self.messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(msg)
                .build()?
                .into(),
        );
        let markup = html! { div class="user-message bg-blue-500 text-white self-end rounded-xl rounded-br-none p-3 max-w-3/4 break-words" { (PreEscaped(markdown::to_html(msg))) } };
        self.send(markup).await?;
        Ok(())
    }

    async fn send_assistant(&mut self, msg: &str) -> anyhow::Result<()> {
        self.messages.push(
            ChatCompletionRequestAssistantMessageArgs::default()
                .content(msg)
                .build()?
                .into(),
        );
        let markup = html! { div #assistant class="assistant-message bg-gray-200 text-black self-start rounded-xl rounded-bl-none p-3 max-w-3/4 break-words" { (PreEscaped(markdown::to_html(msg)))} };
        self.send(markup).await?;
        Ok(())
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        loop {
            match self.recv().await? {
                Some(msg) => {
                    self.process_message(&msg.chat_message).await?;
                }
                None => return Ok(()),
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

                return Ok(());
            } else {
                self.process_function_calls(&assistant_response, &tool_calls)
                    .await?;
            }
        }
    }

    async fn send_tool_calls(
        &mut self,
        calls: Vec<ChatCompletionRequestMessage>,
    ) -> anyhow::Result<()> {
        Ok(self.messages.extend(calls))
    }

    async fn process_function_calls(
        &mut self,
        assistant_response: &str,
        tool_calls: &[ChatCompletionMessageToolCall],
    ) -> anyhow::Result<()> {
        let mut messages = vec![];
        messages.push(
            ChatCompletionRequestAssistantMessageArgs::default()
                .content(assistant_response)
                .tool_calls(Vec::from(tool_calls))
                .build()?
                .into(),
        );
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
            messages.push(
                ChatCompletionRequestToolMessageArgs::default()
                    .content(text_response)
                    .tool_call_id(id)
                    .build()?
                    .into(),
            );
        }
        self.send_tool_calls(messages).await?;
        Ok(())
    }
}
