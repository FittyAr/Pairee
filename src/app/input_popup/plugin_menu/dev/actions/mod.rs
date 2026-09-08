mod browse;
mod build;

pub(super) use browse::{
    handle_option_open_dev_folder, handle_option_open_package_folder,
    handle_option_open_submit_folder, handle_option_select_active_plugin,
};
pub(super) use build::{
    handle_option_init_plugin, handle_option_install_local, handle_option_lint,
    handle_option_package, handle_option_submit,
};

pub(super) fn resolve_active_plugin_path(
    plugin_folder: &str,
    plugins_dev_dir: &std::path::Path,
) -> std::path::PathBuf {
    if std::path::Path::new(plugin_folder).is_absolute() {
        std::path::PathBuf::from(plugin_folder)
    } else {
        plugins_dev_dir.join(plugin_folder)
    }
}
