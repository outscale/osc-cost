use std::{error, time::Duration};

use aws_sdk_s3::{model::Object, Client};
use log::{info, warn};
use std::thread;
use tokio_stream::StreamExt;

use crate::{
    core::{oos::Oos, Resource, Resources},
    VERSION,
};

use super::Input;

const FETCH_WAIT: u64 = 5;

pub type BucketId = String;
const RESOURCE_NAME: &str = "Oos";

pub struct OosBucket {
    objects: Vec<Object>,
}

impl Input {
    async fn list_buckets(&mut self) -> Option<Vec<String>> {
        let client = Client::new(&self.aws_config);
        let req = client.list_buckets();

        // TODO: handle throttling
        let Ok(result) = req.send().await else {
            warn!("warning: error while retrieving the buckets");
            return None;
        };

        let buckets = result.buckets().unwrap_or_default();
        let buckets = buckets
            .iter()
            .filter_map(|b| b.name())
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        Some(buckets)
    }

    async fn list_objects(&mut self, bucket_name: &str) -> Option<Vec<Object>> {
        let client = Client::new(&self.aws_config);
        let req = client.list_objects_v2().prefix("").bucket(bucket_name);

        // TODO: handle throttling
        let mut stream = req.into_paginator().send();

        let mut objects = vec![];
        while let Ok(Some(result)) = stream.try_next().await {
            objects.extend(result.contents().unwrap_or_default().to_vec());
            if result.is_truncated {
                info!("waiting before request other objects");
                thread::sleep(Duration::from_secs(FETCH_WAIT));
            }
        }

        Some(objects)
    }

    #[tokio::main]
    pub async fn fetch_buckets(&mut self) -> Result<(), Box<dyn error::Error>> {
        if self.skip_fetch(RESOURCE_NAME) {
            return Ok(());
        }
        let Some(buckets) = self.list_buckets().await else {
            return Ok(());
        };

        for bucket in &buckets {
            let Some(objects) = self.list_objects(bucket).await else {
                continue;
            };

            self.buckets
                .insert(bucket.to_string(), OosBucket { objects });
        }

        info!("info: fetched {} buckets", self.buckets.len());

        Ok(())
    }

    pub fn fill_resource_oos(&self, resources: &mut Resources) {
        if self.buckets.is_empty() && self.need_default_resource {
            resources.resources.push(Resource::Oos(Oos {
                account_id: self.account_id(),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                ..Default::default()
            }));
        }
        let Some(price_gb_per_month) = self.catalog_entry("TinaOS-OOS", "enterprise", "OOSStorage")
        else {
            warn!("gib price is not defined for oos");
            return;
        };
        for (bucket_id, bucket) in &self.buckets {
            let size = ((bucket
                .objects
                .iter()
                .map(|o| o.size())
                .reduce(|o1, o2| o1 + o2)
                .unwrap_or(0) as f64)
                / 2_f64.powi(30)) as f32;

            let core_resource = Oos {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: self.account_id(),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                resource_id: Some(bucket_id.clone()),
                price_per_hour: None,
                price_per_month: None,
                size_gb: Some(size),
                price_gb_per_month,
                number_files: bucket.objects.len() as u32,
            };
            resources.resources.push(Resource::Oos(core_resource));
        }
    }
}
