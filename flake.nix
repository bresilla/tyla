{
  description = "bevy_mara — reusable glass-themed Bevy + egui editor UI kit, development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    nixgl.url = "github:nix-community/nixGL";
  };

  outputs =
    { self, nixpkgs, rust-overlay, flake-utils, nixgl, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [
          (final: prev: {
            xorg = prev.xorg // {
              libX11 = final.libx11;
              libxcb = final.libxcb;
              libxshmfence = final.libxshmfence;
            };
          })
          (import rust-overlay)
        ];

        pkgs = import nixpkgs {
          inherit system overlays;
          config = {
            allowUnfree = true;
            nvidia.acceptLicense = true;
          };
        };

        nvidiaVersion = let v = builtins.getEnv "NVIDIA_VERSION";
        in if v != "" then v
           else throw "bevy_mara: NVIDIA_VERSION is unset — is direnv loaded and is the NVIDIA driver running?";

        # Upstream nixGL HEAD is incompatible with recent nixpkgs in two ways;
        # patch both here since there is no fixed nixGL release to bump to:
        #   1. It calls `nvidiaDrivers.override { kernel = null; }`, but nixpkgs
        #      removed the `kernel` arg from nvidia-x11/generic.nix (the kernel
        #      module was split out). That throws "unexpected argument 'kernel'".
        #   2. It references the Vulkan ICD as `nvidia_icd.x86_64.json`, but the
        #      driver now installs it as plain `nvidia_icd.json`, so the wrapper
        #      build fails on a missing file.
        nixglPatched = pkgs.runCommand "nixgl-patched" { } ''
          cp -r ${nixgl} $out
          chmod -R +w $out
          sed -i '/kernel = null;/d' $out/nixGL.nix
          sed -i 's#nvidia_icd\.x86_64\.json#nvidia_icd.json#g; s#nvidia_icd\.i686\.json#nvidia_icd.json#g' $out/nixGL.nix
        '';

        nixglPkgs = import "${nixglPatched}/default.nix" {
          inherit pkgs nvidiaVersion;
          nvidiaHash = null;
        };

        nixGLAlias = pkgs.runCommand "nixGL" { } ''
          mkdir -p $out/bin
          ln -s ${nixglPkgs.nixGLNvidia}/bin/nixGLNvidia-${nvidiaVersion} $out/bin/nixGL
        '';
        nixVulkanAlias = pkgs.runCommand "nixVulkan" { } ''
          mkdir -p $out/bin
          ln -s ${nixglPkgs.nixVulkanNvidia}/bin/nixVulkanNvidia-${nvidiaVersion} $out/bin/nixVulkan
        '';

        bevyLibs = with pkgs; [
          alsa-lib
          udev
          vulkan-loader
          libxkbcommon
          wayland
          libx11
          libxcursor
          libxi
          libxrandr
        ];

        # Desktop/web toolchain — the lean default. Android targets are
        # deliberately kept OUT of here so the default shell never drags
        # in the NDK; use `nix develop .#android` for those.
        rustDefault = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rustfmt" "clippy" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # Android cross-compile toolchain (opt-in shell).
        rustAndroid = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rustfmt" "clippy" ];
          targets = [
            "wasm32-unknown-unknown"
            "aarch64-linux-android"
            "armv7-linux-androideabi"
            "x86_64-linux-android"
            "i686-linux-android"
          ];
        };

        # Android SDK + NDK. composeAndroidPackages requires accepting the
        # Android SDK license (handled by config below). Kept minimal:
        # one platform + build-tools (needed by cargo-apk) and one NDK.
        androidEnv = pkgs.androidenv.override { licenseAccepted = true; };
        androidComposition = androidEnv.composeAndroidPackages {
          platformVersions = [ "34" ];
          buildToolsVersions = [ "34.0.0" ];
          includeNDK = true;
        };
        androidSdkRoot = "${androidComposition.androidsdk}/libexec/android-sdk";
      in
      {
        devShells.default = pkgs.mkShell {
          packages = [
            rustDefault
            pkgs.clang
            pkgs.mold
            pkgs.pkg-config

            pkgs.trunk

            (pkgs.python3.withPackages (ps: with ps; [ fonttools brotli ]))

            nixGLAlias
            nixVulkanAlias
            nixglPkgs.nixGLNvidia
            nixglPkgs.nixVulkanNvidia
            nixglPkgs.nixGLIntel
            nixglPkgs.nixVulkanIntel
          ] ++ bevyLibs;

          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath bevyLibs;
          WGPU_VALIDATION = "0";
          WGPU_DEBUG = "0";
        };

        # Opt-in Android cross-compile shell: `nix develop .#android`.
        # Pulls the NDK/SDK (large), so it is intentionally separate from
        # the default shell. cargo-ndk reads ANDROID_NDK_ROOT to locate
        # the cross clang/linker; cargo-apk reads ANDROID_HOME for SDK
        # build-tools when packaging an APK.
        devShells.android = pkgs.mkShell {
          packages = [
            rustAndroid
            pkgs.clang
            pkgs.pkg-config
            pkgs.cargo-ndk
            pkgs.cargo-apk
            pkgs.jdk17
            androidComposition.androidsdk
          ];

          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          ANDROID_HOME = androidSdkRoot;
          ANDROID_SDK_ROOT = androidSdkRoot;

          shellHook = ''
            # NDK path is version-stamped (…/ndk/<ver>); glob it so we are
            # not pinned to a specific NDK revision nixpkgs happens to ship.
            export ANDROID_NDK_ROOT="$(ls -d ${androidSdkRoot}/ndk/* 2>/dev/null | sort | tail -1)"
            export ANDROID_NDK_HOME="$ANDROID_NDK_ROOT"
            echo "android shell: NDK=$ANDROID_NDK_ROOT"
            echo "  compile check:  cargo ndk -t arm64-v8a build -p mara_core"
          '';
        };
      }
    );
}
