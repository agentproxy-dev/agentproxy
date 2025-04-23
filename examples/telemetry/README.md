## Telemetry Example

This example shows how to use the agentgateway to visualize traces and metrics for MCP calls.
This builds upon the [RBAC sample](../rbac).
While you can use tracing without RBAC, this example uses both together to showcase augmenting traces with information from the authenicated user.

### Running the example

```bash
cargo run -- -f examples/telemetry/config.json
```

Let's look at the config to understand what's going on.

In addition to the baseline configuration we had previously, we have a `tracing` section, which tells the proxy how to trace requests.

Here, we configure sending traces to an [OTLP](https://opentelemetry.io/docs/specs/otel/protocol/) endpoint.

We also add two additional tags to the traces:
* `source.user`, which is defined by looking up a field in the JWT claims (as indicated by the `@`).
  Here, we look up the `sub` field which os generally the "user".
* `cluster`, which we define to be a static string.

```json
"tracing": {
  "tracer": {
    "otlp": {
      "endpoint": "http://localhost:4317"
    }
  },
  "tags": {
    "source.user": "@sub",
    "cluster": "static-name"
  }
},
"metrics": {
  "tags": {
    "source.user": "@sub",
    "cluster": "static-name"
  }
},
```

For metrics, we add the same configuration.
Warning: adding unbounded fields into metrics can result in massive costs; ensure only bounded fields are used.

Next we will want to get a tracing backend running.
You can use any OTLP endpoint if you already have one, or run a local [Jaeger](https://www.jaegertracing.io/) instance by running `docker compose up -d`

Now that we have the gateway running, we can use the [mcpinspector](https://github.com/modelcontextprotocol/inspector) to try it out.
```bash
npx @modelcontextprotocol/inspector
```

Now that we have the inspector running, let's connect to the gateway, for this tutorial we'll need to use a JWT token. If we try without we will get an error.
```
eyJhbGciOiJFUzI1NiIsImtpZCI6IlhoTzA2eDhKaldIMXd3a1dreWVFVXhzb29HRVdvRWRpZEVwd3lkX2htdUkiLCJ0eXAiOiJKV1QifQ.eyJhdWQiOiJtZS5jb20iLCJleHAiOjE5MDA2NTAyOTQsImlhdCI6MTc0Mjg2OTUxNywiaXNzIjoibWUiLCJqdGkiOiI3MDViYjM4MTNjN2Q3NDhlYjAyNzc5MjViZGExMjJhZmY5ZDBmYzE1MDNiOGY3YzFmY2I1NDc3MmRiZThkM2ZhIiwibmJmIjoxNzQyODY5NTE3LCJzdWIiOiJtZSJ9.cLeIaiWWMNuNlY92RiCV3k7mScNEvcVCY0WbfNWIvRFMOn_I3v-oqFhRDKapooJZLWeiNldOb8-PL4DIrBqmIQ
```

![Inspector](../rbac/img/connect.png)

Now that we're connected, let's try out the echo tool.

![Echo](../rbac/img/echo.png)

Now we can open the [Jaeger UI](http://localhost:16686/search) and search for our spans:

![Jaeger](./img/jaeger.png)

We can also see the metrics (typically, these would be scraped by Prometheus):

```
$ curl localhost:9091/metrics -s | grep tool_calls_total
tool_calls_total{server="everything",name="echo",cluster="static-name",source.user="me"} 5
```