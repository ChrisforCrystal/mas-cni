use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct PluginConf {
    #[serde(rename = "cniVersion")]
    pub cni_version: String,
    
    pub name: String,
    
    #[serde(rename = "type")]
    pub plugin_type: String,

    // Mascni specifics
    #[serde(default = "default_bridge")]
    pub bridge: String,
    
    #[serde(rename = "isGateway", default = "default_true")]
    pub is_gateway: bool,
    
    #[serde(rename = "ipMasq", default = "default_true")]
    pub ip_masq: bool,

    pub ipam: IPAMConf,
}

#[derive(Debug, Deserialize)]
pub struct IPAMConf {
    #[serde(rename = "type")]
    pub ipam_type: String,
    
    // Capture other IPAM fields (subnet, routes, etc.) to pass through
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

fn default_bridge() -> String {
    "mascni0".to_string()
}

fn default_true() -> bool {
    true
}
