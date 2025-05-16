pub mod db;
pub mod provisioning;
pub mod schema;

pub use db::*;
pub use provisioning::*;

pub fn get_tenant_connection_string(tenant_name: &str) -> String {
    provisioning::get_tenant_connection_string(tenant_name)
}
