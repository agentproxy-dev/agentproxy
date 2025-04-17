<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/agentproxy/agentproxy/refs/heads/main/img/banner-light.svg" alt="agentproxy" width="400">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/agentproxy/agentproxy/refs/heads/main/img/banner-dark.svg" alt="agentproxy" width="400">
    <img alt="agentproxy" src="https://raw.githubusercontent.com/agentproxy/agentproxy/refs/heads/main/img/icon-light.svg">
  </picture>
  <div>
     <a href="https://discord.gg/BdJpzaPjHv">
      <img src="https://img.shields.io/discord/1346225185166065826?style=flat&label=Join%20Discord&color=6D28D9" alt="Discord">
    </a>
    <a href="https://github.com/agentproxy-dev/agentproxy/releases">
      <img src="https://img.shields.io/github/v/release/agentproxy/agentproxy?style=flat&label=Latest%20Release&color=6D28D9" alt="Latest Release">
    </a>
    <a href="https://github.com/agentproxy-dev/agentproxy/actions/workflows/release.yml">
      <img src="https://github.com/agentproxy-dev/agentproxy/actions/workflows/release.yml/badge.svg" alt="Release">
    </a>
  </div>
  <div>
    The first <strong>full featured</strong>, <strong>enterprise-grade</strong> Agent first proxy.
  </div>
</div>

---


**Key Features:**

- [x] **Highly performant:** agentproxy is written in rust, and is designed from the ground up to handle any scale you can throw at it.
- [x] **Security First:** agentproxy includes a robust MCP/A2A focused RBAC system.
- [x] **Multi Tenant:** agentproxy supports multiple tenants, each with their own set of resources and users.
- [x] **Dynamic:** agentproxy supports dynamic configuration updates via xDS, without any downtime.
- [x] **Run Anywhere:** agentproxy can run anywhere, from a single machine to a large scale multi-tenant deployment.
- [x] **Legacy API Support:** agentproxy can transform legacy APIs into MCP resources. Currently supports OpenAPI. (gRPC coming soon)
- [x] **Open Source:** agentproxy is open source, and licensed under the [Apache 2.0 license](https://www.apache.org/licenses/LICENSE-2.0).
<br>


# Getting Started 
**Build**

```bash
cargo build
```

**Run**

Local config file
```bash
cargo run -- -f examples/config/static.json
```

Remote config file
```bash
cargo run -- -f https://raw.githubusercontent.com/agentproxy/agentproxy/main/examples/config/static.json
```
