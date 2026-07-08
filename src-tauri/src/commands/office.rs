use anyhow::Result;
use serde::Deserialize;

use crate::core::{
    docker::CliDocker,
    office_preflight::{self, CliHostPreflight, OfficePreflightResult, Resources},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficePreflightArgs {
    #[serde(default)]
    pub name: Option<String>,
    pub resources: Resources,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeStartProvisioningArgs {
    pub name: String,
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
    pub language: String,
    pub resources: Resources,
    #[serde(rename = "byolAccepted", alias = "byol_accepted")]
    pub byol_accepted: bool,
    #[serde(default, rename = "telemetryOptIn", alias = "telemetry_opt_in")]
    pub telemetry_opt_in: Option<bool>,
    #[serde(default, rename = "adoptionId", alias = "adoption_id")]
    pub adoption_id: Option<String>,
}

pub fn preflight(args: OfficePreflightArgs) -> Result<OfficePreflightResult> {
    let _profile_name = args.name.as_deref().unwrap_or("");
    office_preflight::run_preflight(&args.resources, &CliDocker, &CliHostPreflight)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn office_preflight_args_match_start_provisioning() {
        let preflight: OfficePreflightArgs = serde_json::from_value(serde_json::json!({
            "name": "office",
            "resources": {
                "ramGb": 8,
                "cpuCores": 4,
                "diskGb": 128,
                "storagePath": "/tmp/winbox-office",
                "warningOverride": true
            }
        }))
        .expect("office_preflight args should deserialize");
        let start: OfficeStartProvisioningArgs = serde_json::from_value(serde_json::json!({
            "name": "office",
            "productId": "O365ProPlusRetail",
            "language": "pt-br",
            "resources": {
                "ramGb": 8,
                "cpuCores": 4,
                "diskGb": 128,
                "storagePath": "/tmp/winbox-office",
                "warningOverride": true
            },
            "byolAccepted": true,
            "telemetryOptIn": false,
            "adoptionId": null
        }))
        .expect("office_start_provisioning args should deserialize");

        assert_eq!(preflight.resources, start.resources);

        let snake_case_start: OfficeStartProvisioningArgs =
            serde_json::from_value(serde_json::json!({
                "name": "office",
                "product_id": "O365BusinessRetail",
                "language": "en-us",
                "resources": {
                    "ramGb": 8,
                    "cpuCores": 4,
                    "diskGb": 128
                },
                "byol_accepted": true,
                "telemetry_opt_in": true,
                "adoption_id": "existing"
            }))
            .expect("aliases should remain compatible with command contract");
        assert_eq!(
            snake_case_start.resources.ram_gb,
            preflight.resources.ram_gb
        );
    }
}
