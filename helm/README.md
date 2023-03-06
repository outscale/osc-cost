# Prerequisite
- Kubernetes (>=1.22.0)
- Helm (>=v3.9.3)
- Prometheus (>=0.57.0)
- Grafana (>=6.31.0)

# Deploying osc-cost as prometheus exporter

Please set your ak/sk in osc-cost charts values.yaml:
```yaml
  secret:
    # -- enable secret
    enable: True
    # -- Outscale Access Key
    oscAccessKey: supersecret 
    # -- Outscale Secret Key
    oscSecretKey: topsecret 
    # -- Outscale Region
    oscRegion: region
```
or create your own secret in the same namespace and set in values.yaml:
```yaml
  secret:
    # -- enable secret
    enable: True
```
and create secret:
```bash
 kubectl create secret generic osccost-secret --from-literal=OSC_ACCESS_KEY=supersecret --from-literal=OSC_SECRET_KEY=topsecret --from-literal=OSC_REGION=region -n kube-system
```

This step will deploy osc-cost as prometheus exporter:
```bash
helm install osccost -n kube-system ./osccost/
NAME: osccost
LAST DEPLOYED: Mon Mar  6 09:53:49 2023
NAMESPACE: kube-system
STATUS: deployed
REVISION: 1
TEST SUITE: None
NOTES:
# SPDX-FileCopyrightText: 2023 Outscale SAS
# SPDX-License-Identifier: BSD-3-Clause

Thank you for installing osccost

Your release is name osccost

To learn more about this release, try:
  $ helm status osccost
  $ helm get osccost
```

# Create your own grafana dashboard
You can create your own grafana dashboard using [prometheus query](https://grafana.com/docs/grafana/latest/datasources/prometheus/query-editor/)


# Uninstall osc-cost prometheus exporter
You can uninstall osc-cost:
```bash
helm uninstall osc-cost -n kube-system
```
