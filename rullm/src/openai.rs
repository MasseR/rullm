use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionTool, CreateChatCompletionRequestArgs,
        CreateChatCompletionResponse,
    },
};

use crate::{conf::Conf, mcp::MCP};

pub struct OpenAIClient {
    client: Client<OpenAIConfig>,
    model: String,
    tools: Vec<ChatCompletionTool>,
}

impl OpenAIClient {
    pub async fn build(conf: &Conf, mcp: &MCP) -> anyhow::Result<OpenAIClient> {
        let openai_base = conf
            .llm
            .base_url
            .as_ref()
            .map(|x| x.clone())
            .unwrap_or(String::from("https://api.openai.com/v1"));
        let openai_config = OpenAIConfig::default()
            .with_api_key(&conf.llm.api_key)
            .with_api_base(openai_base);
        let client = Client::with_config(openai_config);
        let model = conf
            .llm
            .model
            .as_ref()
            .cloned()
            .unwrap_or(String::from("gpt-4o"));
        let tools = mcp.list_tools().await?;
        Ok(OpenAIClient {
            client,
            model,
            tools,
        })
    }

    pub async fn chat(
        &self,
        messages: &Vec<ChatCompletionRequestMessage>,
    ) -> anyhow::Result<CreateChatCompletionResponse> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(self.model.clone())
            .messages(messages.clone())
            .tools(self.tools.clone())
            .build()?;
        let response = self.client.chat().create(request).await?;
        Ok(response)
    }
}
