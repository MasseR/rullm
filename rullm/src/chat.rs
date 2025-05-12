use crate::env::Env;
use anyhow::bail;
use async_openai::{
    error::OpenAIError,
    types::{
        ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessage,
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestToolMessage, ChatCompletionRequestToolMessageArgs,
        ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs, CreateChatCompletionResponse,
    },
};
use rmcp::model::{RawContent, RawTextContent};
use rustyline::{DefaultEditor, error::ReadlineError};

pub async fn run(env: Env) -> anyhow::Result<()> {
    let mut rl = DefaultEditor::new()?;
    let mut messages: Vec<ChatCompletionRequestMessage> = vec![];
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let assistant_response = chat(&env, &mut messages, &line).await?;
                println!("{}", &assistant_response);
            }
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}

async fn request_llm(env: &Env, messages: &Vec<ChatCompletionRequestMessage>) -> anyhow::Result<CreateChatCompletionResponse> {
    let model = env.conf.llm.model.as_ref().cloned().unwrap_or(String::from("gpt-4o"));
    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages(messages.clone())
        .tools(env.mcp.list_tools().await?)
        .build()?;
    let response = env.client.chat().create(request).await?;
    Ok(response)
}

// Chat with AI
// Will keep track of message history via the 'messages' field
//
// In case of function calling, there might be more than one request involved
async fn chat(
    env: &Env,
    messages: &mut Vec<ChatCompletionRequestMessage>,
    line: &str,
) -> anyhow::Result<String> {
    let message = user_message(&line)?;
    messages.push(message.into());
    // There's a risk that LLM will keep on calling functions
    let mut limit_counter: i8 = 5;
    loop {
        if limit_counter <= 0 {
            bail!("Too many LLM requests")
        }

        let response = request_llm(&env, &messages).await?;
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
            messages.push(assistant_message(&assistant_response, None)?.into());
            // Early return, no function calls
            return Ok(assistant_response);
        } else {
            // Do the tool calling machinery
            let new_messages =
                process_function_calls(&env, &assistant_response, &tool_calls).await?;
            messages.extend(new_messages);
        }
        limit_counter -= 1;
    }
}

async fn process_function_calls(
    env: &Env,
    assistant_response: &str,
    tool_calls: &[ChatCompletionMessageToolCall],
) -> anyhow::Result<Vec<ChatCompletionRequestMessage>> {
    let mut messages = vec![];
    messages.push(assistant_message(&assistant_response, Some(Vec::from(tool_calls)))?.into());
    for call in Vec::from(tool_calls) {
        let id = call.id;
        let response = env.mcp.call_tool(&call.function).await?;
        let mut text_response = String::new();
        for raw in response.content.into_iter().map(|x| x.raw) {
            match raw {
                RawContent::Text(RawTextContent { text }) => text_response.push_str(&text),
                x => bail!("Unknown response: {:?}", x),
            }
        }
        messages.push(tool_message(&id, &text_response)?.into());
    }
    Ok(messages)
}

fn user_message(msg: &str) -> Result<ChatCompletionRequestUserMessage, OpenAIError> {
    ChatCompletionRequestUserMessageArgs::default()
        .content(String::from(msg))
        .build()
}

fn assistant_message(
    msg: &str,
    tool_call: Option<Vec<ChatCompletionMessageToolCall>>,
) -> Result<ChatCompletionRequestAssistantMessage, OpenAIError> {
    let mut builder = ChatCompletionRequestAssistantMessageArgs::default();
    builder.content(String::from(msg));
    if let Some(calls) = tool_call {
        builder.tool_calls(calls);
    }
    builder.build()
}

fn tool_message(id: &str, content: &str) -> Result<ChatCompletionRequestToolMessage, OpenAIError> {
    ChatCompletionRequestToolMessageArgs::default()
        .content(content)
        .tool_call_id(id)
        .build()
}
