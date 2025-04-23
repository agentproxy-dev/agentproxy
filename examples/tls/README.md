## TLS Example

This example shows how to use the agentgateway to proxy requests to the `everything` tool.

### Running the example

```bash
cargo run -- -f examples/tls/config.json
```

Let's look at the config to understand what's going on. First off we have a listener, which tells the proxy how to listen for incoming requests/connections. In this case we're using the `sse` listener, and then we are specifying our local self-signed certificate and key.

```json
  "listeners": [
    {
      "sse": {
        "address": "0.0.0.0",
        "port": 3000,
        "tls": {
          "cert_pem": {
            "file_path": "examples/tls/certs/cert.pem"
          },
          "key_pem": {
            "file_path": "examples/tls/certs/key.pem"
          }
        }
      }
    }
  ],
```

Next we have a targets section, which tells the proxy how to proxy the incoming requests. In this case we're using the `everything` tool, which is a tool that can do everything.

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
      }
    ]
  }
```

This example currently won't work with the `mcpinspector` as it doesn't support unverified TLS certificates.