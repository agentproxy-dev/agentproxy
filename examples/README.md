## Examples

This directory contains examples of how to use the MCP Proxy. Each example covers a slightly different use-case, if you are unsure where to start, basic is the way to go. The examples increase in complexity, so the recommended order of reading is:

1. basic
2. multiplex
3. rbac
4. tls
5. openapi
6. k8s

### [Basic](basic/README.md)

The basic example is the simplest way to get started with the MCP Proxy. It is a good starting point for understanding the MCP Proxy and how to use it. It is a single listener and target pair.


### [Multiplex](multiplex/README.md)

The multiplex example shows how to use the MCP Proxy to multiplex multiple targets on a single listener.


### [RBAC](rbac/README.md)

The rbac example shows how to use the MCP Proxy to apply RBAC policies to incoming requests. It uses JWT Authentication and RBAC policies to authenticate and authorize incoming requests.


### [TLS](tls/README.md)

The tls example shows how to use the MCP Proxy to terminate TLS connections for added security.


### [OpenAPI](openapi/README.md)

The openapi example shows how to use the MCP Proxy to serve an OpenAPI specification for a given target.


### [K8s](k8s/README.md)

The k8s example shows how to run the MCP proxy in kubernetes.

### [Admin](admin/README.md)

The admin example shows how to configure the MCP proxy via an admin API as opposed to a static config file.















