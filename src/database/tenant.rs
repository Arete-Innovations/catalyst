use std::env;

use crate::meltdown::*;

#[derive(Debug, Clone)]
pub struct TenantConnection {
    pub tenant_name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
}

impl TenantConnection {
    pub fn from_env(tenant_name: String) -> Result<Self, MeltDown> {
        let prefix = "PREFIX_DATABASE_URL";
        let url_template = env::var(prefix).map_err(|e| MeltDown::new(MeltType::EnvironmentError, format!("{} not set: {}", prefix, e)))?;

        let parts: Vec<&str> = url_template.splitn(2, "://").collect();
        if parts.len() != 2 {
            return Err(MeltDown::new(MeltType::ConfigurationError, format!("Invalid database URL format: {}", url_template)));
        }

        let connection_parts: Vec<&str> = parts[1].splitn(2, "@").collect();
        if connection_parts.len() != 2 {
            return Err(MeltDown::new(MeltType::ConfigurationError, format!("Invalid database URL format: {}", url_template)));
        }

        let credentials: Vec<&str> = connection_parts[0].split(":").collect();
        if credentials.len() != 2 {
            return Err(MeltDown::new(MeltType::ConfigurationError, format!("Invalid credentials in database URL: {}", url_template)));
        }

        let location_parts: Vec<&str> = connection_parts[1].split("/").collect();
        if location_parts.len() < 1 {
            return Err(MeltDown::new(MeltType::ConfigurationError, format!("Invalid host in database URL: {}", url_template)));
        }

        let host_port: Vec<&str> = location_parts[0].split(":").collect();
        let (host, port) = if host_port.len() == 2 {
            (host_port[0].to_string(), host_port[1].to_string())
        } else {
            (host_port[0].to_string(), "5432".to_string())
        };

        Ok(TenantConnection {
            tenant_name,
            host,
            port,
            username: credentials[0].to_string(),
            password: credentials[1].to_string(),
        })
    }

    pub fn build_connection_string(&self) -> String {
        // Get the dynamic connection string using the centralized method
        crate::vessel::database::get_tenant_connection_string(&self.tenant_name)
    }
}
