pub struct ServiceBusManager {}

impl ServiceBusManager {
    pub async fn get_azure_ad_token(
        config: &AzureAdConfig,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            config.tenant_id
        );
        let client = reqwest::Client::new();
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &config.client_id),
            ("client_secret", &config.client_secret),
            ("scope", "https://management.azure.com/.default"),
        ];
        let resp = client.post(url).form(&params).send().await?;
        let json: serde_json::Value = resp.json().await?;
        let token = json["access_token"]
            .as_str()
            .ok_or("No access_token in response")?
            .to_string();
        Ok(token)
    }

    pub async fn list_queues_azure_ad(
        config: &AzureAdConfig,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let token = Self::get_azure_ad_token(config).await?;
        let url = format!(
            "https://management.azure.com/subscriptions/{}/resourceGroups/{}/providers/Microsoft.ServiceBus/namespaces/{}/queues?api-version=2017-04-01",
            config.subscription_id, config.resource_group, config.namespace
        );
        let client = reqwest::Client::new();
        let resp = client.get(url).bearer_auth(token).send().await?;
        let json: serde_json::Value = resp.json().await?;
        let mut queues = Vec::new();
        if let Some(arr) = json["value"].as_array() {
            for queue in arr {
                if let Some(name) = queue["name"].as_str() {
                    queues.push(name.to_string());
                }
            }
        }
        Ok(queues)
    }

    pub async fn list_namespaces_azure_ad(
        config: &AzureAdConfig,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let token = Self::get_azure_ad_token(config).await?;
        let url = format!(
            "https://management.azure.com/subscriptions/{}/resourceGroups/{}/providers/Microsoft.ServiceBus/namespaces?api-version=2017-04-01",
            config.subscription_id, config.resource_group
        );
        let client = reqwest::Client::new();
        let resp = client.get(url).bearer_auth(token).send().await?;
        let json: serde_json::Value = resp.json().await?;
        let mut namespaces = Vec::new();
        if let Some(arr) = json["value"].as_array() {
            for ns in arr {
                if let Some(name) = ns["name"].as_str() {
                    namespaces.push(name.to_string());
                }
            }
        }
        Ok(namespaces)
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct AzureAdConfig {
    tenant_id: String,
    client_id: String,
    client_secret: String,
    subscription_id: String,
    resource_group: String,
    pub namespace: String,
}

impl AzureAdConfig {
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
    pub fn client_secret(&self) -> &str {
        &self.client_secret
    }
    pub fn subscription_id(&self) -> &str {
        &self.subscription_id
    }
    pub fn resource_group(&self) -> &str {
        &self.resource_group
    }
    pub fn namespace(&self) -> &str {
        &self.namespace
    }
}
