pub mod registry_pack;
pub mod registry_repo;
pub mod validate;

pub use registry_pack::{package_to_registry, package_to_registry_with_progress};
pub use validate::validate_for_publish;

pub fn package() -> anyhow::Result<()> {
    let path = std::env::current_dir()?;
    let msg = package_to_registry(&path)?;
    println!("{}", msg);
    Ok(())
}
