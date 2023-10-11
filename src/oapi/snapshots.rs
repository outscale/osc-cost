use std::error;

use log::warn;
use outscale_api::{
    apis::snapshot_api::read_snapshots,
    models::{FiltersSnapshot, ReadSnapshotsRequest, ReadSnapshotsResponse},
};

use crate::{
    choose_default,
    core::{snapshots::Snapshot, Resource, Resources},
    VERSION,
};

use super::Input;

pub type SnapshotId = String;
const RESOURCE_NAME: &str = "Snapshot";

impl Input {
    pub fn fetch_snapshots(&mut self) -> Result<(), Box<dyn error::Error>> {
        if self.skip_fetch(RESOURCE_NAME) {
            return Ok(());
        }
        let account_id = match self.account_id() {
            None => {
                warn!("warning: no account_id available... skipping");
                return Ok(());
            }
            Some(account_id) => account_id,
        };
        let filters: FiltersSnapshot = match &self.filters {
            Some(filter) => FiltersSnapshot {
                account_ids: Some(vec![account_id]),
                tag_keys: Some(filter.tag_keys.clone()),
                tag_values: Some(filter.tag_values.clone()),
                tags: Some(filter.tags.clone()),
                ..Default::default()
            },
            None => FiltersSnapshot {
                account_ids: Some(vec![account_id]),
                ..Default::default()
            },
        };
        let request = ReadSnapshotsRequest {
            filters: Some(Box::new(filters)),
            ..Default::default()
        };
        let result: ReadSnapshotsResponse = loop {
            let response = read_snapshots(&self.config, Some(request.clone()));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        let snapshots = match result.snapshots {
            None => {
                warn!("warning: no snapshot available");
                return Ok(());
            }
            Some(snapshots) => snapshots,
        };
        for snapshot in snapshots {
            let snapshot_id = snapshot
                .snapshot_id
                .clone()
                .unwrap_or_else(|| String::from(""));
            self.snapshots.insert(snapshot_id, snapshot);
        }
        warn!("info: fetched {} snapshots", self.snapshots.len());
        Ok(())
    }

    pub fn fill_resource_snapshot(&self, resources: &mut Resources) {
        let Some(price_gb_per_month) =
            self.catalog_entry("TinaOS-FCU", "Snapshot:Usage", "Snapshot")
        else {
            warn!("gib price is not defined for snapshot");
            return;
        };
        let snapshosts = &self.snapshots;

        for (snapshot_id, snapshot) in snapshosts {
            let core_snapshot = Snapshot {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: self.account_id(),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                resource_id: choose_default!(
                    snapshosts,
                    Some("".to_string()),
                    Some(snapshot_id.clone()),
                    self.need_default_resource
                ),
                price_per_hour: choose_default!(
                    snapshosts,
                    Some(0.0),
                    None,
                    self.need_default_resource
                ),
                price_per_month: choose_default!(
                    snapshosts,
                    Some(0.0),
                    None,
                    self.need_default_resource
                ),
                volume_size_gib: choose_default!(
                    snapshosts,
                    Some(0),
                    snapshot.volume_size,
                    self.need_default_resource
                ),
                price_gb_per_month: choose_default!(
                    snapshosts,
                    0.0,
                    price_gb_per_month,
                    self.need_default_resource
                ),
            };
            resources.resources.push(Resource::Snapshot(core_snapshot));
        }
    }
}
