## Multiplex Example

This example shows how to use the agentgateway to proxy requests to the `everything` tool.

### Running the example

```bash
cargo run -- -f examples/multiplex/config.json
```

Let's look at the config to understand what's going on. First off we have a listener, which tells the gateway how to listen for incoming requests/connections. In this case we're using the `sse` listener, which is a simple HTTP listener that listens on port 3000.

```json
  "listeners": [
    {
      "sse": {
        "address": "0.0.0.0",
        "port": 3000
      }
    }
  ],
```

Next we have a targets section, which tells the gateway how to proxy the incoming requests. In this case we're using the `everything` tool, which is a tool that can do everything. However unlike the basic example, we added a second target that will proxy requests to the `everything_else` tool. In a real world scenario, this could be any other MCP compatible server, or even servers which we support translating from MCP.

```json
  "targets": {
    "mcp": [
      {
        "name": "everything",
        "stdio": {
          "cmd": "npx",
          "args": [
            "@modelcontextprotocol/server-everything"
          ]
        }
      },
      {
        "name": "everything_else",
        "stdio": {
          "cmd": "npx",
          "args": [
            "@modelcontextprotocol/server-everything"
          ]
        }
      }
    ]
  }
```

Now that we have the gateway running, we can use the [mcpinspector](https://github.com/modelcontextprotocol/inspector) to try it out.
```bash
npx @modelcontextprotocol/inspector
```
Once the inspector is running, it will present the port that it's running on, and then you can navigate to it in your browser.

![Inspector](./img/connect.png)

Once you're connected, you can navigate to the tools tab and see the available tools. Note that there are now tools from two different servers. The tool list is identical because the servers are the same, but they are different logical targets in the system 

![Tools](./img/tools.png)

Let's try out one of the tools, like `everything:echo`.

![Echo](./img/call_0.png)

That worked! The gateway was able to proxy the request to the `everything` tool and return the response. Now let's try out the `everything_else` tool.

![Echo](./img/call_1.png)

That worked! The gateway was able to proxy the request to the `everything_else` tool and return the response.


