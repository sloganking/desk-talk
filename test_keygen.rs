use reqwest;
use serde::Serialize;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let account_id = "40461088-b3c4-4c48-b4ff-8267dbafd938";
    let license_key = "D5D8FE-381ADB-4E54A0-D2323D-1FAB90-V3";
    let fingerprint = "test-machine-123";

    // Test 1: Validate license
    println!("=== TESTING LICENSE VALIDATION ===");

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

    let client = reqwest::Client::new();
    let url = format!(
        "https://api.keygen.sh/v1/accounts/{}/licenses/actions/validate-key",
        account_id
    );

    let response = client
        .post(&url)
        .header("Content-Type", "application/vnd.api+json")
        .header("Accept", "application/vnd.api+json")
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    let body_text = response.text().await?;

    println!("Validation status: {}", status);
    println!("Validation response: {}", body_text);

    if !status.is_success() {
        println!("\n❌ Validation failed!");
        return Ok(());
    }

    let json: serde_json::Value = serde_json::from_str(&body_text)?;
    let license_id = json["data"]["id"].as_str().unwrap_or("");
    println!("✓ License ID: {}", license_id);

    // Test 2: Machine activation with license KEY as bearer
    println!("\n=== TESTING MACHINE ACTIVATION (LICENSE KEY AS BEARER) ===");

    #[derive(Serialize)]
    struct MachineRequest<'a> {
        data: MachineData<'a>,
    }

    #[derive(Serialize)]
    struct MachineData<'a> {
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
        license: LicenseRelationship<'a>,
    }

    #[derive(Serialize)]
    struct LicenseRelationship<'a> {
        data: LicenseData<'a>,
    }

    #[derive(Serialize)]
    struct LicenseData<'a> {
        #[serde(rename = "type")]
        type_field: &'a str,
        id: &'a str,
    }

    let machine_body = MachineRequest {
        data: MachineData {
            type_field: "machines",
            attributes: MachineAttributes {
                fingerprint,
                name: "Test Machine",
            },
            relationships: MachineRelationships {
                license: LicenseRelationship {
                    data: LicenseData {
                        type_field: "licenses",
                        id: license_key,
                    },
                },
            },
        },
    };

    let machine_url = format!("https://api.keygen.sh/v1/accounts/{}/machines", account_id);

    let machine_response = client
        .post(&machine_url)
        .header("Content-Type", "application/vnd.api+json")
        .header("Accept", "application/vnd.api+json")
        .bearer_auth(license_key)
        .json(&machine_body)
        .send()
        .await?;

    let machine_status = machine_response.status();
    let machine_body_text = machine_response.text().await?;

    println!("Machine activation status: {}", machine_status);
    println!("Machine activation response: {}", machine_body_text);

    if machine_status.is_success() {
        println!("\n✅ SUCCESS! Machine activated with license key as bearer auth.");
    } else {
        println!("\n❌ Failed with license key as bearer. Trying license ID...");

        // Test 3: Try with license ID as bearer
        let machine_body2 = MachineRequest {
            data: MachineData {
                type_field: "machines",
                attributes: MachineAttributes {
                    fingerprint: "test-machine-456",
                    name: "Test Machine 2",
                },
                relationships: MachineRelationships {
                    license: LicenseRelationship {
                        data: LicenseData {
                            type_field: "licenses",
                            id: license_id,
                        },
                    },
                },
            },
        };

        let machine_response2 = client
            .post(&machine_url)
            .header("Content-Type", "application/vnd.api+json")
            .header("Accept", "application/vnd.api+json")
            .bearer_auth(license_key)
            .json(&machine_body2)
            .send()
            .await?;

        let status2 = machine_response2.status();
        let body2 = machine_response2.text().await?;

        println!("\nWith license ID in body:");
        println!("Status: {}", status2);
        println!("Response: {}", body2);

        if status2.is_success() {
            println!("\n✅ SUCCESS! Use license ID in body, license KEY as bearer!");
        }
    }

    Ok(())
}
