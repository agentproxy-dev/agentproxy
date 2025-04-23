## Kubernetes Example

This example shows how to deploy the agentgateway in a Kubernetes cluster with a static config.

### Running the Agent Gateway


First let's apply our config to the cluster.
```bash
kubectl create -n agentgateway configmap agentgateway-config --from-file=config.json=examples/k8s/config.json
```

Now we can deploy the Agent Gateway .
```bash
kubectl apply -n agentgateway -f examples/k8s/manifest.yaml
```


### Deploying the example

```bash
kubectl apply -n agentgateway -f examples/k8s/manifest.yaml
```

Once all of the pods are up and running, you can test the gateway by port-forwarding the gateway pod, and then using the mcp inspector. In the first shell run:
```bash
kubectl port-forward -n agentgateway deploy/agentgateway 3000
```

In the second shell run:
```bash
npx npx @modelcontextprotocol/inspector
```

If everything worked correctly, you should be able to list tools and see the following:
![Inspector](./img/tools.png)

Let's try out one of the tools, like `everything:add`.

![Echo](./img/call.png)

To clean this up simply delete the namespace that this example ran in.

```bash
kubectl delete namespace agentgateway
```

