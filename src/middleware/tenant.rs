use serde::Serialize;

#[derive(Serialize, Debug, Default)]
pub struct TenantContext {
    pub tenant_name: String,
}

#[derive(Serialize, Debug)]
pub struct TenantData<T: Serialize> {
    pub tenant: TenantContext,
    #[serde(flatten)]
    pub data: T,
}

impl TenantContext {
    pub fn new(tenant_name: &str) -> Self {
        Self { tenant_name: tenant_name.to_string() }
    }

    pub fn with_data<T: Serialize>(self, data: T) -> TenantData<T> {
        TenantData { tenant: self, data }
    }
}

impl<T: Serialize> TenantData<T> {
    pub fn new(tenant_name: &str, data: T) -> Self {
        Self {
            tenant: TenantContext::new(tenant_name),
            data,
        }
    }
}
