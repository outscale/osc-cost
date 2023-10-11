# osc-cost
[![Project Sandbox](https://docs.outscale.com/fr/userguide/_images/Project-Sandbox-yellow.svg)](https://docs.outscale.com/en/userguide/Open-Source-Projects.html)

osc-cost allows Outscale users to estimate their cloud costs.

### DISCLAMER

This program only provides a cost estimation of the current account state.
Only official bills provided by Outscale will represent your consumption.
Read license for more details.

# Features

- Data sources:
  - [Outscale API](https://docs.outscale.com/api)
  - JSON
- Supported resources:
  - Virtual Machines (tina types, aws-compatible types, licenses)
  - Volumes
  - Public Ips
  - Snapshots (🚨 Warning: snapshot computation is currently known to be over-priced.)
  - Load Balancers
  - Flexible GPU
  - VPN Connections
  - Outscale Object Storage (🚨 Warning: Oos computation can take a long time, use `--skip-resource Oos` to disable this computation.)
  - Nat Services
- Output formats:
  - Current cost per hour
  - Current cost per month
  - Current cost per year
  - Json (line-delimited JSON document)
  - Human
  - Open Document Spreadsheet (Ods)
  - Prometheus



# Installation

Go to release and download latest binary.

# Configuration

You will need `.osc/config.json` file in you home folder. osc-cost takes `default` profile if not specified.
Example of `config.json`:
```json
{
  "default": {
    "access_key": "YoUrAcCeSsKeY",
    "secret_key": "YoUrSeCrEtKeY",
    "region": "eu-west-2"
  }
}
```

# Run

Here are few examples with different output formats. Note that `json` format will provide the most detailed output.

```
osc-cost --format=human # default
Summary:
╭───────────────────────┬──────────────╮
│ Account Id            ┆ 620346218618 │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Total price per hour  ┆ 2.2062643€   │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Total price per month ┆ 1610.5729€   │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Total price per year  ┆ 19326.875€   │
╰───────────────────────┴──────────────╯

Details:
╭───────────────┬───────┬──────────────────────┬───────────────────────┬──────────────────────╮
│ Resource Type ┆ Count ┆ Total price per hour ┆ Total price per month ┆ Total price per year │
╞═══════════════╪═══════╪══════════════════════╪═══════════════════════╪══════════════════════╡
│ Snapshot      ┆ 23    ┆ 0.03164384€          ┆ 23.1€                 ┆ 277.2€               │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Vm            ┆ 9     ┆ 1.7939999€           ┆ 1309.6199€            ┆ 15715.438€           │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ LoadBalancer  ┆ 2     ┆ 0.06€                ┆ 43.8€                 ┆ 525.6€               │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ NatServices   ┆ 2     ┆ 0.1€                 ┆ 73€                   ┆ 876€                 │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Volume        ┆ 12    ┆ 0.20554796€          ┆ 150.04999€            ┆ 1800.5999€           │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ PublicIp      ┆ 7     ┆ 0.015€               ┆ 10.95€                ┆ 131.4€               │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Oos           ┆ 5     ┆ 0.00007237231€       ┆ 0.05283179€           ┆ 0.63398147€          │
╰───────────────┴───────┴──────────────────────┴───────────────────────┴──────────────────────╯
```

```
osc-cost --format=hour
0.42
```

```
osc-cost --format=month
150.91
```

```
osc-cost --format=json
{"resource_type":"Vm","osc_cost_version":"0.1.0","account_id":"509075394552","read_date_rfc3339":"2022-11-24T11:15:50.665643413+00:00","region":"eu-west-2","resource_id":"i-e51434a6","price_per_hour":0.044,"price_per_month":32.12,"vm_type":"tinav4.c1r1p2","vm_vcpu_gen":"4","vm_core_performance":"high","vm_image":"ami-bb490c7e","vm_vcpu":1,"vm_ram_gb":1,"price_vcpu_per_hour":0.039,"price_ram_gb_per_hour":0.005,"price_box_per_hour":0.0,"price_product_per_ram_gb_per_hour":0.0,"price_product_per_cpu_per_hour":0.0,"price_product_per_vm_per_hour":0.0}
{"resource_type":"Volume","osc_cost_version":"0.1.0","account_id":"509075394552","read_date_rfc3339":"2022-11-24T11:15:50.665643413+00:00","region":"eu-west-2","resource_id":"vol-9e99bad9","price_per_hour":0.02321918,"price_per_month":16.95,"volume_type":"io1","volume_size":15,"volume_iops":1500,"price_gb_per_month":0.13,"price_iops_per_month":0.01}
{"resource_type":"PublicIp","osc_cost_version":"0.1.0","account_id":"509075394552","read_date_rfc3339":"2022-11-24T11:15:50.665643413+00:00","region":"eu-west-2","resource_id":"eipalloc-2e5f8e4f","price_per_hour":0.0,"price_per_month":0.0,"price_non_attached":null,"price_first_ip":0.0,"price_next_ips":null}
...
```

```
osc-cost -n --format=prometheus
# HELP FlexibleGpu_price_hour FlexibleGpu price by hour
# TYPE FlexibleGpu_price_hour gauge
FlexibleGpu_price_hour{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="FlexibleGpu"} 0
# HELP FlexibleGpu_price_month FlexibleGpu price by month
# TYPE FlexibleGpu_price_month gauge
FlexibleGpu_price_month{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="FlexibleGpu"} 0
# HELP LoadBalancer_price_hour LoadBalancer price by hour
# TYPE LoadBalancer_price_hour gauge
LoadBalancer_price_hour{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="LoadBalancer"} 0
# HELP LoadBalancer_price_month LoadBalancer price by month
# TYPE LoadBalancer_price_month gauge
LoadBalancer_price_month{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="LoadBalancer"} 0
# HELP NatServices_price_hour NatServices price by hour
# TYPE NatServices_price_hour gauge
NatServices_price_hour{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="NatServices"} 0
# HELP NatServices_price_month NatServices price by month
# TYPE NatServices_price_month gauge
NatServices_price_month{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="NatServices"} 0
# HELP Oos_price_hour Oos price by hour
# TYPE Oos_price_hour gauge
Oos_price_hour{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="Oos"} 0
# HELP Oos_price_month Oos price by month
# TYPE Oos_price_month gauge
Oos_price_month{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="Oos"} 0
# HELP PublicIp_price_hour PublicIp price by hour
# TYPE PublicIp_price_hour gauge
PublicIp_price_hour{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="PublicIp"} 0
# HELP PublicIp_price_month PublicIp price by month
# TYPE PublicIp_price_month gauge
PublicIp_price_month{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="PublicIp"} 0
# HELP Snapshot_price_hour Snapshot price by hour
# TYPE Snapshot_price_hour gauge
Snapshot_price_hour{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="Snapshot"} 0
# HELP Snapshot_price_month Snapshot price by month
# TYPE Snapshot_price_month gauge
Snapshot_price_month{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="Snapshot"} 0
# HELP Vm_price_hour Vm price by hour
# TYPE Vm_price_hour gauge
Vm_price_hour{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="Vm"} 0
# HELP Vm_price_month Vm price by month
# TYPE Vm_price_month gauge
Vm_price_month{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="Vm"} 0
# HELP Volume_price_hour Volume price by hour
# TYPE Volume_price_hour gauge
Volume_price_hour{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="Volume"} 0
# HELP Volume_price_month Volume price by month
# TYPE Volume_price_month gauge
Volume_price_month{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="Volume"} 0
# HELP Vpn_price_hour Vpn price by hour
# TYPE Vpn_price_hour gauge
Vpn_price_hour{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="Vpn"} 0
# HELP Vpn_price_month Vpn price by month
# TYPE Vpn_price_month gauge
Vpn_price_month{account_id="040667503696",osc_cost_version="0.3.3",region="cloudgouv-eu-west-1",resource_id="",resource_type="Vpn"} 0
```


> **_NOTE:_** The next feature is still in beta

The tools can also be used to see the drift between osc-cost estimation and what have been actually recorded. Here are the steps to do that:
- store the output of osc-cost in a json
  ```
  osc-cost --format json --output account.json
  ```
- freeze the account during **one day**
- the next day 
  ```
    osc-cost --compute-drift --from-date "$(date "+%Y-%m-%d" --date='-1day') --to-date $(date "+%Y-%m-%d") --input account.json
  ```

You will have the details of the drift.
```
╭───────────────┬──────────┬────────┬───────╮
│ Resource Type ┆ Osc-cost ┆ Digest ┆ Drift │
╞═══════════════╪══════════╪════════╪═══════╡
│ Volume        ┆ 1.18     ┆ 1.18   ┆ 0%    │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┤
│ Oos           ┆ 0.01     ┆ 0.01   ┆ -5%   │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┤
│ Snapshot      ┆ 1.25     ┆ 0.62   ┆ 101%  │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┤
│ Vm            ┆ 34.01    ┆ 34.01  ┆ 0%    │
╰───────────────┴──────────┴────────┴───────╯
```

# Contributing

Check [contributing documentation](CONTRIBUTING.md).

# Release

1. Update chart version (if necessary) in `helm/osccost/Chart.yaml` and osc-cost version in `helm/osccost/values.yaml`

2. Tag the release
```
git tag -a vX.X.X -m "vX.X.X"
```

3. Make the release on Github

# License

> Copyright Outscale SAS
>
> BSD-3-Clause

This project is compliant with [REUSE](https://reuse.software/).
