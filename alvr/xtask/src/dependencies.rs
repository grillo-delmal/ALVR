use crate::command;
use alvr_filesystem as afs;
use std::{fs, path::Path};
use xshell::{cmd, Shell};

pub fn choco_install(sh: &Shell, packages: &[&str]) -> Result<(), xshell::Error> {
    cmd!(
        sh,
        "powershell Start-Process choco -ArgumentList \"install {packages...} -y\" -Verb runAs -Wait"
    )
    .run()
}

pub fn prepare_ffmpeg_windows(deps_path: &Path) {
    command::download_and_extract_zip(
        &format!(
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/{}",
            "ffmpeg-n5.1-latest-win64-gpl-shared-5.1.zip"
        ),
        deps_path,
    )
    .unwrap();

    fs::rename(
        deps_path.join("ffmpeg-n5.1-latest-win64-gpl-shared-5.1"),
        deps_path.join("ffmpeg"),
    )
    .unwrap();
}

pub fn prepare_windows_deps(skip_admin_priv: bool) {
    let sh = Shell::new().unwrap();

    let deps_path = afs::deps_dir().join("windows");
    sh.remove_path(&deps_path).ok();
    sh.create_dir(&deps_path).unwrap();

    if !skip_admin_priv {
        choco_install(
            &sh,
            &[
                "zip",
                "unzip",
                "llvm",
                "vulkan-sdk",
                "wixtoolset",
                "pkgconfiglite",
            ],
        )
        .unwrap();
    }

    prepare_ffmpeg_windows(&deps_path);
}

pub fn prepare_linux_deps(nvenc_flag: bool) {
    let sh = Shell::new().unwrap();

    let deps_path = afs::deps_dir().join("linux");
    sh.remove_path(&deps_path).ok();
    sh.create_dir(&deps_path).unwrap();

    build_ffmpeg_linux(nvenc_flag, &deps_path);
}

pub fn build_ffmpeg_linux(nvenc_flag: bool, deps_path: &Path) {
    let sh = Shell::new().unwrap();

    let ffmpeg = &afs::workspace_dir().join("ffmpeg.zip");
    command::unzip(&sh, &ffmpeg, deps_path).unwrap();

    let final_path = deps_path.join("ffmpeg");

    fs::rename(deps_path.join("FFmpeg-n6.0"), &final_path).unwrap();

    let flags = [
        "--enable-gpl",
        "--enable-version3",
        "--enable-static",
        "--disable-programs",
        "--disable-doc",
        "--disable-avdevice",
        "--disable-avformat",
        "--disable-swresample",
        "--disable-swscale",
        "--disable-postproc",
        "--disable-network",
        "--disable-everything",
        "--enable-encoder=h264_vaapi",
        "--enable-encoder=hevc_vaapi",
        "--enable-encoder=av1_vaapi",
        "--enable-hwaccel=h264_vaapi",
        "--enable-hwaccel=hevc_vaapi",
        "--enable-hwaccel=av1_vaapi",
        "--enable-filter=scale_vaapi",
        "--enable-vulkan",
        "--enable-libdrm",
        "--enable-pic",
        "--fatal-warnings",
    ];
    let install_prefix = format!("--prefix={}", final_path.join("alvr_build").display());
    let config_vars = r#"-Wl"#;

    let _push_guard = sh.push_dir(final_path);
    let _env_vars = sh.push_env("LDSOFLAGS", config_vars);

    // Patches ffmpeg for workarounds and patches that have yet to be unstreamed
    let ffmpeg_command = "for p in ../../../alvr/xtask/patches/*; do patch -p1 < $p; done";
    cmd!(sh, "bash -c {ffmpeg_command}").run().unwrap();

    if nvenc_flag {
        /*
           Describing Nvidia specific options --nvccflags:
           nvcc from CUDA toolkit version 11.0 or higher does not support compiling for 'compute_30' (default in ffmpeg)
           52 is the minimum required for the current CUDA 11 version (Quadro M6000 , GeForce 900, GTX-970, GTX-980, GTX Titan X)
           https://arnon.dk/matching-sm-architectures-arch-and-gencode-for-various-nvidia-cards/
           Anyway below 50 arch card don't support nvenc encoding hevc https://developer.nvidia.com/nvidia-video-codec-sdk (Supported devices)
           Nvidia docs:
           https://docs.nvidia.com/video-technologies/video-codec-sdk/ffmpeg-with-nvidia-gpu/#commonly-faced-issues-and-tips-to-resolve-them
        */
        #[cfg(target_os = "linux")]
        {
            let codec_header_version = "12.1.14.0";
            let temp_download_dir = deps_path.join("dl_temp");
            let codec_header = &afs::workspace_dir().join("nv-codec-headers.zip");
            command::unzip(&sh, &codec_header, &temp_download_dir).unwrap();

            let header_dir = deps_path.join("nv-codec-headers");
            let header_build_dir = header_dir.join("build");
            fs::rename(
                temp_download_dir.join(format!("nv-codec-headers-n{codec_header_version}")),
                &header_dir,
            )
            .unwrap();
            fs::remove_dir_all(temp_download_dir).unwrap();
            {
                let make_header_cmd =
                    format!("make install PREFIX='{}'", header_build_dir.display());
                let _header_push_guard = sh.push_dir(&header_dir);
                cmd!(sh, "bash -c {make_header_cmd}").run().unwrap();
            }

            let nvenc_flags = &[
                "--enable-encoder=h264_nvenc",
                "--enable-encoder=hevc_nvenc",
                "--enable-encoder=av1_nvenc",
            ];

            let env_vars = format!(
                "PKG_CONFIG_PATH='{}'",
                header_build_dir.join("lib/pkgconfig").display()
            );
            let flags_combined = flags.join(" ");
            let nvenc_flags_combined = nvenc_flags.join(" ");

            let command = format!(
                "{env_vars} ./configure {install_prefix} {flags_combined} {nvenc_flags_combined}"
            );

            cmd!(sh, "bash -c {command}").run().unwrap();
        }
    } else {
        cmd!(sh, "./configure {install_prefix} {flags...}")
            .run()
            .unwrap();
    }

    let nproc = cmd!(sh, "nproc").read().unwrap();
    cmd!(sh, "make -j{nproc}").run().unwrap();
    cmd!(sh, "make install").run().unwrap();
}

pub fn prepare_macos_deps() {
    let sh = Shell::new().unwrap();
}

fn get_android_openxr_loaders(only_khronos_loader: bool) {
    fn get_openxr_loader(name: &str, url: &str, source_dir: &str) {
        let sh = Shell::new().unwrap();
        let temp_dir = afs::build_dir().join("temp_download");
        sh.remove_path(&temp_dir).ok();
        sh.create_dir(&temp_dir).unwrap();
        let destination_dir = afs::deps_dir().join("android_openxr/arm64-v8a");
        fs::create_dir_all(&destination_dir).unwrap();

        command::download_and_extract_zip(url, &temp_dir).unwrap();
        fs::copy(
            temp_dir.join(source_dir).join("libopenxr_loader.so"),
            destination_dir.join(format!("libopenxr_loader{name}.so")),
        )
        .unwrap();
        fs::remove_dir_all(&temp_dir).ok();
    }

    get_openxr_loader(
        "",
        &format!(
            "https://github.com/KhronosGroup/OpenXR-SDK-Source/releases/download/{}",
            "release-1.0.34/openxr_loader_for_android-1.0.34.aar",
        ),
        "prefab/modules/openxr_loader/libs/android.arm64-v8a",
    );

    if !only_khronos_loader {
        get_openxr_loader(
            "_quest1",
            "https://securecdn.oculus.com/binaries/download/?id=7577210995650755", // Version 64
            "OpenXR/Libs/Android/arm64-v8a/Release",
        );

        get_openxr_loader(
            "_pico",
            "https://sdk.picovr.com/developer-platform/sdk/PICO_OpenXR_SDK_220.zip",
            "libs/android.arm64-v8a",
        );

        get_openxr_loader(
            "_yvr",
            "https://developer.yvrdream.com/yvrdoc/sdk/openxr/yvr_openxr_mobile_sdk_2.0.0.zip",
            "yvr_openxr_mobile_sdk_2.0.0/OpenXR/Libs/Android/arm64-v8a",
        );

        get_openxr_loader(
            "_lynx",
            "https://portal.lynx-r.com/downloads/download/16", // version 1.0.0
            "jni/arm64-v8a",
        );
    }
}

pub fn build_android_deps(skip_admin_priv: bool, all_targets: bool, only_khronos_loader: bool) {
    let sh = Shell::new().unwrap();

    if cfg!(windows) && !skip_admin_priv {
        choco_install(&sh, &["unzip", "llvm"]).unwrap();
    }

    cmd!(sh, "rustup target add aarch64-linux-android")
        .run()
        .unwrap();
    if all_targets {
        cmd!(sh, "rustup target add armv7-linux-androideabi")
            .run()
            .unwrap();
        cmd!(sh, "rustup target add x86_64-linux-android")
            .run()
            .unwrap();
        cmd!(sh, "rustup target add i686-linux-android")
            .run()
            .unwrap();
    }
    cmd!(sh, "cargo install cargo-ndk cbindgen").run().unwrap();
    cmd!(
        sh,
        "cargo install --git https://github.com/zarik5/cargo-apk cargo-apk"
    )
    .run()
    .unwrap();

    get_android_openxr_loaders(only_khronos_loader);
}
