use crate::cata_log;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapPhase {
    PreConfig,
    PostConfig,
    PreDatabase,
    PostDatabase,
    PreSparks,
    PostSparks,
    PostBootstrap,
}

impl BootstrapPhase {
    pub fn name(&self) -> &'static str {
        match self {
            BootstrapPhase::PreConfig => "pre_config",
            BootstrapPhase::PostConfig => "post_config",
            BootstrapPhase::PreDatabase => "pre_database",
            BootstrapPhase::PostDatabase => "post_database",
            BootstrapPhase::PreSparks => "pre_sparks",
            BootstrapPhase::PostSparks => "post_sparks",
            BootstrapPhase::PostBootstrap => "post_bootstrap",
        }
    }
}

pub async fn run_custom_bootstrap(phase: BootstrapPhase) -> Result<(), Box<dyn std::error::Error>> {
    cata_log!(Debug, format!("Running custom bootstrap for {} phase", phase.name()));
    
    match phase {
        BootstrapPhase::PreConfig => {
            pre_config_hook().await?;
        }
        BootstrapPhase::PostConfig => {
            post_config_hook().await?;
        }
        BootstrapPhase::PreDatabase => {
            pre_database_hook().await?;
        }
        BootstrapPhase::PostDatabase => {
            post_database_hook().await?;
        }
        BootstrapPhase::PreSparks => {
            pre_sparks_hook().await?;
        }
        BootstrapPhase::PostSparks => {
            post_sparks_hook().await?;
        }
        BootstrapPhase::PostBootstrap => {
            post_bootstrap_hook().await?;
        }
    }
    
    Ok(())
}

async fn pre_config_hook() -> Result<(), Box<dyn std::error::Error>> {
    cata_log!(Debug, "Pre-config hook: Add your custom logic here");
    Ok(())
}

async fn post_config_hook() -> Result<(), Box<dyn std::error::Error>> {
    cata_log!(Debug, "Post-config hook: Add your custom logic here");
    Ok(())
}

async fn pre_database_hook() -> Result<(), Box<dyn std::error::Error>> {
    cata_log!(Debug, "Pre-database hook: Add your custom logic here");
    Ok(())
}

async fn post_database_hook() -> Result<(), Box<dyn std::error::Error>> {
    cata_log!(Debug, "Post-database hook: Add your custom logic here");
    Ok(())
}

async fn pre_sparks_hook() -> Result<(), Box<dyn std::error::Error>> {
    cata_log!(Debug, "Pre-sparks hook: Add your custom logic here");
    Ok(())
}

async fn post_sparks_hook() -> Result<(), Box<dyn std::error::Error>> {
    cata_log!(Debug, "Post-sparks hook: Add your custom logic here");
    Ok(())
}

async fn post_bootstrap_hook() -> Result<(), Box<dyn std::error::Error>> {
    cata_log!(Debug, "Post-bootstrap hook: Add your custom logic here");
    Ok(())
}