<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/agentgateway/agentgateway/refs/heads/main/img/banner-light.svg" alt="agentgateway" width="400">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/agentgateway/agentgateway/refs/heads/main/img/banner-dark.svg" alt="agentgateway" width="400">
    <img alt="agentgateway" src="https://raw.githubusercontent.com/agentgateway/agentgateway/refs/heads/main/img/banner-light.svg">
  </picture>
  <div>
    <a href="https://opensource.org/licenses/Apache-2.0">
      <img src="https://img.shields.io/badge/License-Apache2.0-brightgreen.svg?style=flat" alt="License: Apache 2.0">
    </a>
    <a href="https://github.com/agentgateway/agentgateway">
      <img src="https://img.shields.io/github/stars/agentgateway/agentgateway.svg?style=flat&logo=github&label=Stars" alt="Stars">
    </a>
    <a href="https://discord.gg/BdJpzaPjHv">
      <img src="https://img.shields.io/discord/1346225185166065826?style=flat&label=Join%20Discord&color=6D28D9" alt="Discord">
    </a>
    <a href="https://github.com/agentgateway/agentgateway/releases">
      <img src="https://img.shields.io/github/v/release/agentgateway/agentgateway?style=flat&label=Latest%20Release&color=6D28D9" alt="Latest Release">
    </a>
    <a href="https://deepwiki.com/agentgateway/agentgateway"><img src="https://deepwiki.com/badge.svg" alt="Ask DeepWiki"></a>
    <a href='https://codespaces.new/agentgateway/agentgateway'>
      <img src='https://github.com/codespaces/badge.svg' alt='Open in Github Codespaces' style='max-width: 100%;' height="20">
    </a>
  </div>
  <div>
    The <strong>first complete</strong> connectivity solution for Agentic AI enables both agent-to-agent and agent-to-tool communication across any environment.
  </div>
</div>

---

Agentgateway is an open source data plane optimized for agentic AI connectivity within or across any agent framework or environment. Agentgateway provides drop-in security, observability, and governance for agent-to-agent and agent-to-tool communication and supports leading interoperable protocols, including [Agent2Agent (A2A)](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/) and [Model Context Protocol (MCP)](https://modelcontextprotocol.io/introduction).

<br> 
<div align="center">
  <img alt="agentgateway UI" src="img/architecture.svg" width="600">
</div>
<br> 

**Key Features:**

- [x] **Highly performant:** agentgateway is written in rust, and is designed from the ground up to handle any scale you can throw at it.
- [x] **Security First:** agentgateway includes a robust MCP/A2A focused RBAC system.
- [x] **Multi Tenant:** agentgateway supports multiple tenants, each with their own set of resources and users.
- [x] **Dynamic:** agentgateway supports dynamic configuration updates via xDS, without any downtime.
- [x] **Run Anywhere:** agentgateway can run anywhere with any agent framework, from a single machine to a large scale multi-tenant deployment.
- [x] **Legacy API Support:** agentgateway can transform legacy APIs into MCP resources. Currently supports OpenAPI. (gRPC coming soon)
<br>

# Getting Started 

To get started with agentgateway, please check out the [Getting Started Guide](https://agentgateway.dev/docs/quickstart ).

## Build from Source

Requirements:
- Rust 1.86+
- npm 10+

Build the agentgateway UI:

```bash
cd ui
npm install
npm run build
```

Build the agentgateway binary:

```bash
cd ..
make build
```

If you encounter an authentication error to the schemars repo in GitHub, try set `CARGO_NET_GIT_FETCH_WITH_CLI=true` and rerun `make build`.

Run the agentgateway binary:

```bash
./target/release/agentgateway
```
Open your browser and navigate to `http://localhost:19000/ui` to see the agentgateway UI.

<div align="center">
  <img alt="agentgateway UI" src="img/UI-homepage.png" width="600">
</div>

# Contributors

Thanks to all contributors who are helping to make kagent better.

<a href="https://github.com/agentgateway/agentgateway/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=agentgateway/agentgateway" />
</a>


## Star History

<a href="https://www.star-history.com/#agentgateway/agentgateway&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=agentgateway/agentgateway&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=agentgateway/agentgateway&type=Date" />
   <img alt="Star history of agentgateway/agentgateway over time" src="https://api.star-history.com/svg?repos=agentgateway/agentgateway&type=Date" />
 </picture>
</a>