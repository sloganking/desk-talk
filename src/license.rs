use anyhow::{Context, Result};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::config::KeygenConfig;

const KEYGEN_API_BASE: &str = "https://api.keygen.sh/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseData {
    pub id: String,
    pub status: Option<String>,
    pub plan: Option<String>,
    pub expires_at: Option<String>,
    pub max_machines: Option<u32>,
    pub machines_used: Option<u32>,
    pub key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineData {
    pub id: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub license: LicenseData,
    pub machine: Option<MachineData>,
}

pub struct KeygenClient {
    config: KeygenConfig,
    client: reqwest::Client,
}

impl KeygenClient {
    pub fn new(config: KeygenConfig) -> Result<Self> {
        let client = reqwest::Client::builder().user_agent("DeskTalk").build()?;
        Ok(Self { config, client })
    }

    pub fn config(&self) -> &KeygenConfig {
        &self.config
    }

    pub fn account_id(&self) -> &str {
        &self.config.account_id
    }

    pub fn product_id(&self) -> &str {
        &self.config.product_id
    }

    pub fn trial_policy(&self) -> &str {
        &self.config.policy_trial
    }

    pub fn pro_policy(&self) -> &str {
        &self.config.policy_pro
    }

    pub async fn validate_license(
        &self,
        license_key: &str,
        fingerprint: &str,
    ) -> Result<ValidationResult> {
        #[derive(Serialize)]
        struct ValidateRequest<'a> {
            meta: ValidateMeta<'a>,
        }

        #[derive(Serialize)]
        struct ValidateMeta<'a> {
            key: &'a str,
            scope: ValidateScope<'a>,
        }

        #[derive(Serialize)]
        struct ValidateScope<'a> {
            fingerprint: &'a str,
        }

        let body = ValidateRequest {
            meta: ValidateMeta {
                key: license_key,
                scope: ValidateScope { fingerprint },
            },
        };

        let url = format!(
            "{}/accounts/{}/licenses/actions/validate-key",
            KEYGEN_API_BASE, self.config.account_id
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/vnd.api+json")
            .header("Accept", "application/vnd.api+json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let body_text = response.text().await?;

        println!("Keygen validation response status: {}", status);
        println!("Keygen validation response body: {}", body_text);

        match status {
            StatusCode::OK => {
                let json: serde_json::Value = serde_json::from_str(&body_text)?;
                parse_validation_json(json)
            }
            StatusCode::UNPROCESSABLE_ENTITY | StatusCode::NOT_FOUND => {
                anyhow::bail!("Invalid license key")
            }
            _ => anyhow::bail!(
                "License validation failed with status {}: {}",
                status,
                body_text
            ),
        }
    }

    pub async fn activate_machine(
        &self,
        license_key: &str,
        license_id: &str,
        fingerprint: &str,
        name: &str,
    ) -> Result<MachineData> {
        println!("Activating machine with fingerprint: {}", fingerprint);

        #[derive(Serialize)]
        struct Request<'a> {
            data: MachineRequestData<'a>,
        }

        #[derive(Serialize)]
        struct MachineRequestData<'a> {
            #[serde(rename = "type")]
            type_field: &'a str,
            attributes: MachineAttributes<'a>,
            relationships: MachineRelationships<'a>,
        }

        #[derive(Serialize)]
        struct MachineAttributes<'a> {
            fingerprint: &'a str,
            name: &'a str,
        }

        #[derive(Serialize)]
        struct MachineRelationships<'a> {
            license: MachineLicenseRelationship<'a>,
        }

        #[derive(Serialize)]
        struct MachineLicenseRelationship<'a> {
            data: MachineLicenseData<'a>,
        }

        #[derive(Serialize)]
        struct MachineLicenseData<'a> {
            #[serde(rename = "type")]
            type_field: &'a str,
            id: &'a str,
        }

        let body = Request {
            data: MachineRequestData {
                type_field: "machines",
                attributes: MachineAttributes { fingerprint, name },
                relationships: MachineRelationships {
                    license: MachineLicenseRelationship {
                        data: MachineLicenseData {
                            type_field: "licenses",
                            id: license_id,
                        },
                    },
                },
            },
        };

        let url = format!(
            "{}/accounts/{}/machines",
            KEYGEN_API_BASE, self.config.account_id
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/vnd.api+json")
            .header("Accept", "application/vnd.api+json")
            .header("Authorization", format!("License {}", license_key))
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let body_text = response.text().await?;
        println!("Keygen machine activation status: {}", status);
        println!("Keygen machine activation body: {}", body_text);

        match status {
            StatusCode::CREATED | StatusCode::OK => {
                let json: serde_json::Value = serde_json::from_str(&body_text)?;
                parse_machine_json(json)
            }
            StatusCode::UNPROCESSABLE_ENTITY => {
                anyhow::bail!("Machine activation failed: {}", body_text)
            }
            status => anyhow::bail!(
                "Machine activation failed with status {}: {}",
                status,
                body_text
            ),
        }
    }
}

fn parse_validation_json(json: serde_json::Value) -> Result<ValidationResult> {
    let data_value = json.get("data").context("missing data field")?;

    let validation = match data_value {
        serde_json::Value::Array(arr) => arr.get(0).cloned().context("missing validation entry")?,
        serde_json::Value::Object(_) => data_value.clone(),
        _ => anyhow::bail!("unexpected data format returned by Keygen"),
    };

    let included = json
        .get("included")
        .and_then(|i| i.as_array())
        .cloned()
        .unwrap_or_default();

    let license_id = validation
        .get("id")
        .and_then(|id| id.as_str())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let license_attributes = validation.get("attributes").or_else(|| {
        included
            .iter()
            .find(|item| {
                item.get("type").and_then(|t| t.as_str()) == Some("licenses")
                    && item.get("id").and_then(|id| id.as_str()) == Some(license_id.as_str())
            })
            .and_then(|license| license.get("attributes"))
    });

    let meta = validation.get("meta").cloned().unwrap_or_default();
    let key = meta
        .get("key")
        .and_then(|k| k.as_str())
        .map(|s| s.to_string());

    // Extract plan from policy relationship
    let policy_id = validation
        .get("relationships")
        .and_then(|r| r.get("policy"))
        .and_then(|p| p.get("data"))
        .and_then(|d| d.get("id"))
        .and_then(|id| id.as_str())
        .map(|s| s.to_string());

    // Determine plan name from policy (you can customize this logic)
    let plan = if let Some(ref pid) = policy_id {
        // Check if it contains "trial" or matches trial policy pattern
        if pid.to_lowercase().contains("trial") {
            Some("Trial".to_string())
        } else {
            Some("Pro".to_string())
        }
    } else {
        // Fallback: try to get from attributes
        license_attributes
            .and_then(|attrs| attrs.get("plan"))
            .and_then(|p| p.as_str())
            .map(|s| s.to_string())
    };

    let license = LicenseData {
        id: license_id.clone(),
        status: license_attributes
            .and_then(|attrs| attrs.get("status"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string()),
        plan,
        expires_at: license_attributes
            .and_then(|attrs| attrs.get("expiresAt").or_else(|| attrs.get("expires_at")))
            .and_then(|e| e.as_str())
            .map(|s| s.to_string()),
        max_machines: license_attributes
            .and_then(|attrs| {
                attrs
                    .get("maxMachines")
                    .or_else(|| attrs.get("max_machines"))
            })
            .and_then(|m| m.as_u64())
            .map(|v| v as u32),
        machines_used: license_attributes
            .and_then(|attrs| {
                attrs
                    .get("machinesInUse")
                    .or_else(|| attrs.get("machines_used"))
            })
            .and_then(|m| m.as_u64())
            .map(|v| v as u32),
        key,
    };

    let machine = included
        .iter()
        .find(|item| item.get("type").and_then(|t| t.as_str()) == Some("machines"))
        .and_then(|machine| {
            let id = machine.get("id")?.as_str()?.to_string();
            let attributes = machine.get("attributes")?;
            let fingerprint = attributes
                .get("fingerprint")
                .and_then(|f| f.as_str())
                .unwrap_or_default()
                .to_string();
            Some(MachineData { id, fingerprint })
        })
        .or_else(|| {
            validation
                .get("relationships")
                .and_then(|r| r.get("machine"))
                .and_then(|m| m.get("data"))
                .and_then(|d| d.get("id"))
                .and_then(|id| id.as_str())
                .map(|id| MachineData {
                    id: id.to_string(),
                    fingerprint: meta
                        .get("scope")
                        .and_then(|s| s.get("fingerprint"))
                        .and_then(|f| f.as_str())
                        .unwrap_or_default()
                        .to_string(),
                })
        });

    Ok(ValidationResult { license, machine })
}

fn parse_machine_json(json: serde_json::Value) -> Result<MachineData> {
    let data = json.get("data").context("missing data")?.clone();
    let attributes = data
        .get("attributes")
        .context("missing attributes")?
        .clone();
    Ok(MachineData {
        id: data
            .get("id")
            .and_then(|id| id.as_str())
            .unwrap_or_default()
            .to_string(),
        fingerprint: attributes
            .get("fingerprint")
            .and_then(|f| f.as_str())
            .unwrap_or_default()
            .to_string(),
    })
}
