fn main() {
    slint_build::compile("ui/app.slint").expect("failed to compile ui/app.slint");

    // exe にアイコンと Common Controls v6 のマニフェストを埋め込む（ファイルダイアログが XP 風にならないように）
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=windows/qota.rc");
        println!("cargo:rerun-if-changed=windows/qota.exe.manifest");
        embed_resource::compile("windows/qota.rc", embed_resource::NONE)
            .manifest_required()
            .expect("failed to embed windows/qota.rc");
    }
}
