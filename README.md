<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/agentgateway/agentgateway/refs/heads/main/img/banner-light.svg" alt="agentgateway" width="400">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/agentgateway/agentgateway/refs/heads/main/img/banner-dark.svg" alt="agentgateway" width="400">
    <img alt="agentgateway" src="https://raw.githubusercontent.com/agentgateway/agentgateway/refs/heads/main/img/banner-light.svg">
  </picture>
  <div>
     <a href="https://discord.gg/BdJpzaPjHv">
      <img src="https://img.shields.io/discord/1346225185166065826?style=flat&label=Join%20Discord&color=6D28D9" alt="Discord">
    </a>
    <a href="https://github.com/agentgateway/agentgateway/releases">
      <img src="https://img.shields.io/github/v/release/agentgateway/agentgateway?style=flat&label=Latest%20Release&color=6D28D9" alt="Latest Release">
    </a>
    <a href="https://github.com/agentgateway/agentgateway/actions/workflows/release.yml">
      <img src="https://github.com/agentgateway/agentgateway/actions/workflows/release.yml/badge.svg" alt="Release">
    </a>
  </div>
  <div>
    The first <strong>full featured</strong>, <strong>enterprise-grade</strong> Agent first proxy.
  </div>
</div>

---


**Key Features:**

- [x] **Highly performant:** agentgateway is written in rust, and is designed from the ground up to handle any scale you can throw at it.
- [x] **Security First:** agentgateway includes a robust MCP/A2A focused RBAC system.
- [x] **Multi Tenant:** agentgateway supports multiple tenants, each with their own set of resources and users.
- [x] **Dynamic:** agentgateway supports dynamic configuration updates via xDS, without any downtime.
- [x] **Run Anywhere:** agentgateway can run anywhere, from a single machine to a large scale multi-tenant deployment.
- [x] **Legacy API Support:** agentgateway can transform legacy APIs into MCP resources. Currently supports OpenAPI. (gRPC coming soon)
- [x] **Open Source:** agentgateway is open source, and licensed under the [Apache 2.0 license](https://www.apache.org/licenses/LICENSE-2.0).
<br>


# Getting Started 
**Build**

```bash
cargo build
```

**Run**

Local config file
```bash
cargo run -- -f examples/basic/config.json
```

Remote config file
```bash
cargo run -- -f https://raw.githubusercontent.com/agentgateway/agentgateway/main/examples/basic/config.json
```
