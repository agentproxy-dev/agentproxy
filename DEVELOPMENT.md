# Local Development

For instructions on how to run everything locally, see the [DEVELOPMENT.md](DEVELOPMENT.md) file.

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

