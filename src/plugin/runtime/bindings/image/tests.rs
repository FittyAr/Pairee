//! `pairee.image` with free and jailed policies.

use super::*;

/// A 40×20 RGB PNG at `path`.
fn png(path: &Path) {
    image::RgbImage::from_pixel(40, 20, image::Rgb([200, 10, 10]))
        .save(path)
        .unwrap();
}

fn lua_with(policy: FsPolicy) -> Lua {
    let lua = Lua::new();
    lua.globals()
        .set("image", bind(&lua, Arc::new(policy)).unwrap())
        .unwrap();
    lua
}

#[test]
fn info_and_show_for_a_trusted_plugin() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.png");
    png(&file);
    let lua = lua_with(FsPolicy::new(Path::new("demo"), true, false));
    lua.globals()
        .set("file", file.to_string_lossy().as_ref())
        .unwrap();
    let (w, h, format, is_image): (u32, u32, String, bool) = lua
        .load(
            r#"local i = image.info(file)
               return i.w, i.h, i.format, type(image.show(file)) == "userdata""#,
        )
        .eval()
        .unwrap();
    assert_eq!((w, h, format.as_str()), (40, 20, "png"));
    assert!(is_image);
    let kind: String = lua
        .load(r#"local _, e = image.info(file .. ".missing") return e.kind"#)
        .eval()
        .unwrap();
    assert_eq!(kind, "NotFound");
}

#[test]
fn untrusted_plugins_show_only_the_previewed_file_or_their_own() {
    let root = tempfile::tempdir().unwrap();
    let _redirect = crate::config::paths::test_root::redirect(root.path());
    let plugin_dir = root.path().join("demo.pairee");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    let outside = tempfile::tempdir().unwrap();
    let file = outside.path().join("b.png");
    png(&file);
    let policy = FsPolicy::new(&plugin_dir, false, false);
    std::fs::create_dir_all(&policy.data_dir).unwrap();
    let data_dir = policy.data_dir.clone();
    let lua = lua_with(policy);
    lua.globals()
        .set("file", file.to_string_lossy().as_ref())
        .unwrap();
    let kind: String = lua
        .load(r#"local _, e = image.show(file) return e.kind"#)
        .eval()
        .unwrap();
    assert_eq!(kind, "PermissionDenied");

    lua.set_app_data(PreviewTarget(file.clone()));
    let (shown, w): (bool, u32) = lua
        .load(r#"return image.show(file) ~= nil, image.info(file).w"#)
        .eval()
        .unwrap();
    assert!(shown);
    assert_eq!(w, 40);
    // The preview exception does not let precache read the pixels.
    let thumb = data_dir.join("thumb.png");
    lua.globals()
        .set("thumb", thumb.to_string_lossy().as_ref())
        .unwrap();
    let (ok, kind): (bool, String) = lua
        .load(r#"local ok, e = image.precache(file, thumb) return ok, e.kind"#)
        .eval()
        .unwrap();
    assert!(!ok);
    assert_eq!(kind, "PermissionDenied");

    // Its own files can be precached into its data directory.
    let own = plugin_dir.join("own.png");
    png(&own);
    lua.globals()
        .set("own", own.to_string_lossy().as_ref())
        .unwrap();
    let ok: bool = lua
        .load(r#"return image.precache(own, thumb, { max_width = 10 })"#)
        .eval()
        .unwrap();
    assert!(ok);
    let (w, h) = image::image_dimensions(&thumb).unwrap();
    assert_eq!((w, h), (10, 5));

    // Bounds larger than the source never enlarge it.
    let ok: bool = lua
        .load(r#"return image.precache(own, thumb, { max_width = 4294967295, max_height = 60000 })"#)
        .eval()
        .unwrap();
    assert!(ok);
    assert_eq!(image::image_dimensions(&thumb).unwrap(), (40, 20));
}

#[test]
fn files_over_the_size_cap_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("big.png");
    let f = std::fs::File::create(&file).unwrap();
    f.set_len(QUICK_VIEW_MAX_BYTES + 1).unwrap();
    let lua = lua_with(FsPolicy::new(Path::new("demo"), true, false));
    lua.globals()
        .set("file", file.to_string_lossy().as_ref())
        .unwrap();
    let kind: String = lua
        .load(r#"local _, e = image.info(file) return e.kind"#)
        .eval()
        .unwrap();
    assert_eq!(kind, "FileTooLarge");
}
