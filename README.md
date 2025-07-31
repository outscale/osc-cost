# osc-cost

[![Project Sandbox](https://docs.outscale.com/fr/userguide/_images/Project-Sandbox-yellow.svg)](https://docs.outscale.com/en/userguide/Open-Source-Projects.html)
[![](https://dcbadge.limes.pink/api/server/HUVtY5gT6s?style=flat&theme=default-inverted)](https://discord.gg/HUVtY5gT6s)

<p align="center">
  <img alt="Terminal Icon" src="https://img.icons8.com/ios-filled/100/console.png" width="100px">
</p>

---

## 🌐 Links

* 📘 Outscale API: [docs.outscale.com/api](https://docs.outscale.com/api)
* 📦 Helm chart: [osc-cost](https://github.com/outscale/osc-cost/tree/main/helm)
* 🐳 Docker Compose: [docker-compose.yaml](https://github.com/outscale/osc-cost/blob/main/helm/docker-compose.yaml)
* 🤝 Contribution Guide: [CONTRIBUTING.md](./CONTRIBUTING.md)
* 🔧 Prometheus Exporter: [prometheus\_exporter](https://docs.rs/prometheus_exporter/latest/prometheus_exporter/)
* 💬 Join us on [Discord](https://discord.gg/YOUR_INVITE_CODE)

---

## 📄 Table of Contents

* [Overview](#-overview)
* [Project Status](#-project-status)
* [Requirements](#-requirements)
* [Installation](#-installation)
* [Configuration](#-configuration)
* [Usage](#-usage)
* [Prometheus Exporter](#-prometheus-exporter)
* [Drift Analysis (Beta)](#-drift-analysis-beta)
* [Deployment](#-deployment)
* [Contributing](#-contributing)
* [Release Process](#-release-process)
* [License](#-license)

---

## 🧭 Overview

**osc-cost** is a command-line utility that estimates current cloud costs for an Outscale account by analyzing live resource states.

It supports multiple output formats and can also export metrics to Prometheus or compare estimated costs against digest-based billing (experimental).

---

## 🚧 Project Status

> ⚠️ This project is in **sandbox** status and under active development.
> Cost estimations are approximations and may differ from official billing.
> Only official invoices from OUTSCALE are authoritative.

---

## ✅ Requirements

* An OUTSCALE account with access to the API
* `~/.osc/config.json` for credentials
* Linux/macOS shell (tested with Bash)
* Prometheus (optional, for metric export)

---

## 🔨 Installation

Download the latest binary from the [GitHub Releases](https://github.com/outscale/osc-cost/releases) page.

Make it executable:

```bash
chmod +x osc-cost
mv osc-cost /usr/local/bin/
```

---

## 🛠 Configuration

The tool expects credentials in `~/.osc/config.json`.

### Example config:

```json
{
  "default": {
    "access_key": "YOUR_ACCESS_KEY",
    "secret_key": "YOUR_SECRET_KEY",
    "region": "eu-west-2"
  }
}
```

To use a different profile, use the `--profile` flag.

---

## 🚀 Usage

### Estimate costs (default format: human-readable)

```bash
osc-cost
```

### Output options

```bash
osc-cost --format=human        # human-friendly output
osc-cost --format=json         # detailed structured output
osc-cost --format=ods          # ODS spreadsheet
osc-cost --format=prometheus   # Prometheus format
osc-cost --format=hour         # Only price per hour
osc-cost --format=month        # Only price per month
```

### Skip expensive resources

```bash
osc-cost --skip-resource Oos
```

---

## 📊 Drift Analysis (Beta)

Compare cost estimations with actual usage from digest:

### Step 1 – Export estimation

```bash
osc-cost --format=json --output account.json
```

### Step 2 – Freeze the account for a day

### Step 3 – Compare costs the next day

```bash
osc-cost --compute-drift \
  --from-date "$(date -d '-1 day' +%Y-%m-%d)" \
  --to-date "$(date +%Y-%m-%d)" \
  --input account.json
```

### Example output

```
╭───────────────┬──────────┬────────┬───────╮
│ Resource Type ┆ Osc-cost ┆ Digest ┆ Drift │
╞═══════════════╪══════════╪════════╪═══════╡
│ Volume        ┆ 1.18     ┆ 1.18   ┆ 0%    │
│ Snapshot      ┆ 1.25     ┆ 0.62   ┆ 101%  │
╰───────────────┴──────────┴────────┴───────╯
```

---

## 📈 Prometheus Exporter

Export estimated prices in Prometheus format:

```bash
osc-cost --format=prometheus -n
```

A serde formatter is used to expose metrics in a simple text format.

---

## 🚢 Deployment

### With Helm

Use the [osc-cost Helm chart](https://github.com/outscale/osc-cost/tree/main/helm) for Kubernetes deployment.

### With Docker Compose

```bash
docker-compose -f helm/docker-compose.yaml up
```

### On Kubernetes (Kind or RKE)

You can deploy with any Kubernetes setup.

---

## 🤝 Contributing

We welcome your contributions!

Please read the [CONTRIBUTING.md](./CONTRIBUTING.md) guide.

---

## 🚀 Release Process

1. Update `Chart.yaml` and `values.yaml` in `helm/osccost/`
2. Tag a release:

```bash
git tag -a vX.X.X -m "vX.X.X"
```

3. Push the tag and publish the release on GitHub.

---

## 📜 License

**osc-cost** is licensed under the BSD 3-Clause License.
© Outscale SAS
This project is compliant with the [REUSE Specification](https://reuse.software/)
