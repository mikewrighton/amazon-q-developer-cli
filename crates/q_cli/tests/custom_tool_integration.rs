// Import necessary dependencies for our test
use std::process::Command;

use assert_cmd::prelude::*;
use serde_json::json;
use tempfile::TempDir;
use tokio::fs;

mod common;

#[tokio::test]
async fn test_custom_tool_initialization() {
    // Create a temporary directory for our test files
    let temp_dir = TempDir::new().unwrap();

    // Create mock MCP server script
    let mock_server_path = temp_dir.path().join("mock_mcp_server.sh");
    let mock_server_content = r#"#!/bin/bash
# Simple mock MCP server that responds to JSON-RPC requests

while read -r line; do
  # Parse the JSON-RPC request
  request_id=$(echo "$line" | jq -r '.id')
  method=$(echo "$line" | jq -r '.method')
  
  if [ "$method" = "initialize" ]; then
    # Respond to initialize request
    echo "{\"jsonrpc\":\"2.0\",\"id\":$request_id,\"result\":{\"capabilities\":{\"tools\":true}}}"
  elif [ "$method" = "tools/list" ]; then
    # Respond with tool list
    echo "{\"jsonrpc\":\"2.0\",\"id\":$request_id,\"result\":{\"tools\":[{\"name\":\"hello_world\",\"description\":\"Says hello world\",\"inputSchema\":{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\",\"description\":\"Name to greet\"}}}}]}}"
  elif [ "$method" = "tools/call" ]; then
    # Extract the tool name and arguments
    tool_name=$(echo "$line" | jq -r '.params.name')
    name_arg=$(echo "$line" | jq -r '.params.arguments.name // "World"')
    
    if [ "$tool_name" = "hello_world" ]; then
      # Respond to hello_world tool call
      echo "{\"jsonrpc\":\"2.0\",\"id\":$request_id,\"result\":{\"output\":\"Hello, $name_arg!\"}}"
    else
      # Unknown tool
      echo "{\"jsonrpc\":\"2.0\",\"id\":$request_id,\"error\":{\"code\":-32601,\"message\":\"Unknown tool: $tool_name\"}}"
    fi
  else
    # Unknown method
    echo "{\"jsonrpc\":\"2.0\",\"id\":$request_id,\"error\":{\"code\":-32601,\"message\":\"Method not found\"}}"
  fi
done
"#;
    fs::write(&mock_server_path, mock_server_content).await.unwrap();

    // Make the script executable
    let status = Command::new("chmod")
        .arg("+x")
        .arg(&mock_server_path)
        .status()
        .expect("Failed to make mock server executable");
    assert!(status.success());

    // Create MCP config file
    let config_path = temp_dir.path().join("mcp_config.json");
    let config = json!({
        "mcpServers": {
            "mock_server": {
                "command": mock_server_path.to_str().unwrap(),
                "args": []
            }
        }
    });

    fs::write(&config_path, config.to_string()).await.unwrap();

    // Set the config path in the environment for the test
    std::env::set_var("FIG_SETTINGS_MCP_CONFIG", config_path.to_str().unwrap());

    // Now we'll run the CLI with a command that would use our mock tool
    let mut cmd = common::cli();
    cmd.arg("chat")
        .arg("--verbose")
        .arg("Use the hello_world tool to say hello to Integration Test")
        .assert()
        .success();
}
