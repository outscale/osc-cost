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
  - Snapshots
- Output formats:
  - Current cost per hour
  - Current cost per month
  - Json (line-delimited JSON document)
  - CSV

🚨 Warning: snapshot computation is currently known to be over-priced.

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
osc-cost --format=hour # default
0.42
```

```
osc-cost --format=month
150.91
```

```
osc-cost --format=csv
resource_type,osc_cost_version,account_id,read_date_rfc3339,region,resource_id,price_per_hour,price_per_month,vm_type,vm_vcpu_gen,vm_core_performance,vm_image,vm_vcpu,vm_ram_gb,price_vcpu_per_hour,price_ram_gb_per_hour,price_box_per_hour,price_product_per_ram_gb_per_hour,price_product_per_cpu_per_hour,price_product_per_vm_per_hour
Vm,0.1.0,509075394552,2022-11-24T11:14:31.605623096+00:00,eu-west-2,i-682fc9e7,0.044999998,32.85,tinav5.c1r1p1,5,highest,ami-bb490c7e,1,1,0.04,0.005,0.0,0.0,0.0,0.0
...
```

```
osc-cost --format=json
{"resource_type":"Vm","osc_cost_version":"0.1.0","account_id":"509075394552","read_date_rfc3339":"2022-11-24T11:15:50.665643413+00:00","region":"eu-west-2","resource_id":"i-e51434a6","price_per_hour":0.044,"price_per_month":32.12,"vm_type":"tinav4.c1r1p2","vm_vcpu_gen":"4","vm_core_performance":"high","vm_image":"ami-bb490c7e","vm_vcpu":1,"vm_ram_gb":1,"price_vcpu_per_hour":0.039,"price_ram_gb_per_hour":0.005,"price_box_per_hour":0.0,"price_product_per_ram_gb_per_hour":0.0,"price_product_per_cpu_per_hour":0.0,"price_product_per_vm_per_hour":0.0}
{"resource_type":"Volume","osc_cost_version":"0.1.0","account_id":"509075394552","read_date_rfc3339":"2022-11-24T11:15:50.665643413+00:00","region":"eu-west-2","resource_id":"vol-9e99bad9","price_per_hour":0.02321918,"price_per_month":16.95,"volume_type":"io1","volume_size":15,"volume_iops":1500,"price_gb_per_month":0.13,"price_iops_per_month":0.01}
{"resource_type":"PublicIp","osc_cost_version":"0.1.0","account_id":"509075394552","read_date_rfc3339":"2022-11-24T11:15:50.665643413+00:00","region":"eu-west-2","resource_id":"eipalloc-2e5f8e4f","price_per_hour":0.0,"price_per_month":0.0,"price_non_attached":null,"price_first_ip":0.0,"price_next_ips":null}
...
```

# Contributing

Check [contributing documentation](CONTRIBUTING.md).

# License

> Copyright Outscale SAS
>
> BSD-3-Clause

This project is compliant with [REUSE](https://reuse.software/).
