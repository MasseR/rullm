use std::error::Error;

use async_openai::types::{
    ChatCompletionTool, ChatCompletionToolArgs, FunctionCall, FunctionObject,
};
use rmcp::{
    RoleClient, ServiceExt,
    model::{CallToolRequestParam, CallToolResult, JsonObject, Tool},
    serde_json::{self, Value},
    service::RunningService,
    transport::TokioChildProcess,
};
use tokio::process::Command;

use crate::args::Args;

pub struct MCP {
    client: RunningService<RoleClient, ()>,
}

impl MCP {
    pub async fn build(Args { mcp_client, .. }: &Args) -> Result<MCP, Box<dyn Error>> {
        let client = ().serve(TokioChildProcess::new(Command::new(mcp_client).arg(""))?).await?;
        Ok(MCP { client })
    }
    pub async fn list_tools(&self) -> Result<Vec<ChatCompletionTool>, Box<dyn Error>> {
        let tools = self
            .client
            .list_all_tools()
            .await?
            .into_iter()
            .map(tool_to_function)
            .collect::<Result<Vec<ChatCompletionTool>, Box<dyn Error>>>()?;
        Ok(tools)
    }
    pub async fn call_tool(&self, tool: &FunctionCall) -> Result<CallToolResult, Box<dyn Error>> {
        Ok(self.client.call_tool(function_to_tool(tool)?).await?)
    }
}

fn function_to_tool(function: &FunctionCall) -> Result<CallToolRequestParam, Box<dyn Error>> {
    if let Value::Object(obj) = serde_json::from_str(&function.arguments)? {
        Ok(CallToolRequestParam {
            name: function.name.clone().into(),
            arguments: Some(obj),
        })
    } else {
        Err(Box::from(format!(
            "Couldn't parse {:?}",
            &function.arguments
        )))
    }
}

fn tool_to_function(tool: Tool) -> Result<ChatCompletionTool, Box<dyn Error>> {
    let obj: &JsonObject = &tool.input_schema;
    let parameters: Option<Value> = if obj.contains_key("properties") {
        Some(Value::Object(obj.clone()))
    } else {
        None
    };
    let x = ChatCompletionToolArgs::default()
        .function(FunctionObject {
            name: tool.name.to_string(),
            description: Some(tool.description.to_string()),
            parameters,
            strict: None,
        })
        .build()?;
    Ok(x)
}
