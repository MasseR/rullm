use anyhow::{anyhow, bail};
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

use crate::args::Conf;

pub struct MCP {
    client: RunningService<RoleClient, ()>,
}

impl MCP {
    pub async fn build(
        Conf {
            executables,
            environment,
            ..
        }: &Conf,
    ) -> anyhow::Result<MCP> {
        // I'm for now only taking one item here
        // The data format supports multiple but I haven't yet gotten multiple servers, nor have
        // I figured out how to create multiple clients. The APIs say something about peering
        // but not familiar enough yet
        let mcp_client = executables.get("mealie").ok_or(anyhow!("Missing mealie"));
        let mut cmd = Command::new(mcp_client?);
        for (key, value) in environment {
            cmd.env(key, value);
        }
        let client = ().serve(TokioChildProcess::new(&mut cmd)?).await?;
        Ok(MCP { client })
    }
    pub async fn list_tools(&self) -> anyhow::Result<Vec<ChatCompletionTool>> {
        let tools = self
            .client
            .list_all_tools()
            .await?
            .into_iter()
            .map(tool_to_function)
            .collect::<anyhow::Result<Vec<ChatCompletionTool>>>()?;
        Ok(tools)
    }
    pub async fn call_tool(&self, tool: &FunctionCall) -> anyhow::Result<CallToolResult> {
        Ok(self.client.call_tool(function_to_tool(tool)?).await?)
    }
}

fn function_to_tool(function: &FunctionCall) -> anyhow::Result<CallToolRequestParam> {
    if let Value::Object(obj) = serde_json::from_str(&function.arguments)? {
        Ok(CallToolRequestParam {
            name: function.name.clone().into(),
            arguments: Some(obj),
        })
    } else {
        bail!("Couldn't parse {:?}", &function.arguments)
    }
}

fn tool_to_function(tool: Tool) -> anyhow::Result<ChatCompletionTool> {
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
