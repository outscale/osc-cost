# Osc-cost

## Osc-cost prometheus exporter output

But what is prometheus ([Prometheus][Prometheus])

We create a prometheus exporter ([Prometheus-Exporter][Prometheus-Exporter])

To have something simple to manipulate we create a serde for prometheus. ([Serde][Serde])


## How to deploy with docker-compose

You can deploy with helm chart ([osc-cost-chart][osc-cost-chart])

You can also deploy with docker-compose ([docker-compose][docker-compose])

#### Prerequisite
- docker
- docker-compose

#### Build you image

```bash
docker build -t <name>/<tag> . 
```
### Replace osc-cost image in docker-compose

```yaml
services:
...
  osc-cost:
    image: <name>/<tag>

```

### Have prometheus and prometheus exporter up

```bash
docker-compose up -d
```

## How to deploy with helm

### Prequisite

- helm
- kubernetes
- prometheus

### Helm

To deploy with helm, you can use:
```bash
helm upgrade --install osccost osccost \
  --repo 'git+https://github.com/outscale/osc-cost@helm/osccost?ref=v0.4.2&sparse=0' \
  --namespace osccost \
  --create-namespace \
  --set osccost.serviceMonitor.relabelings[0].targetLabel=account \
  --set osccost.serviceMonitor.relabelings[0].replacement=my-account \
  --set osccost.secret.oscAccessKey=xxxxxxxxxxxxxxxxxxxx \
  --set osccost.secret.oscSecretKey=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx \
  --set osccost.secret.oscRegion=eu-west-2 \
  --set osccost.ingress.enable=false \
  --set osccost.deployment.containers.resources.cpu.limits=1 \
  --set osccost.deployment.containers.resources.memory.limits=2Gi \
  --set osccost.deployment.containers.pullPolicy=IfNotPresent \
  --set osccost.deployment.containers.osccostExtraParams='-n --skip-resource Oos'
```

### Connect to prometheus

Open your favorite web browser in http://127.0.0.1:9090.


## Environment

You need to have a config file.

Test with an account with several cloud object. 

For prometheus, you can use docker-compose file in the projet to test with prometheus.

Or you can test with k8s cluster.

To test with k8s cluster, you can use kind but for me i create a cluster with one worker and one master using this project ([osc-k8s-rke-cluster][osc-k8s-rke-cluster])

A account on outscale on eu-west-2/cloud-gouv or us-east-2. If you want on another region, please create omi on those region.

<!-- References -->

[Prometheus]: https://prometheus.io/
[Prometheus-Exporter]: https://docs.rs/prometheus_exporter/latest/prometheus_exporter/
[Serde]: https://serde.rs/
[docker-compose]: https://github.com/outscale/osc-cost/blob/main/helm/docker-compose.yaml
[osc-k8s-rke-cluster]: https://github.com/outscale/osc-k8s-rke-cluster
[osc-cost-chart]: https://github.com/outscale/osc-cost/blob/main/helm/README.md
