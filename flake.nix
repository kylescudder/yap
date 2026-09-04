{
  description = "Yap local-first voice dictation";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in {
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          python = pkgs.python3.withPackages (ps: [ ps.pygobject3 ]);
          runtimePath = pkgs.lib.makeBinPath [
            pkgs.curl
            pkgs.hyprland
            pkgs.llama-cpp
            pkgs.wireplumber
            pkgs.wl-clipboard
            pkgs.whisper-cpp
            pkgs.wtype
          ];
          typelibPath = pkgs.lib.makeSearchPath "lib/girepository-1.0" [
            pkgs.glib
            pkgs.graphene
            pkgs.gtk3
            pkgs.gtk4
            pkgs.gtk4-layer-shell
            pkgs.libayatana-appindicator
          ];
          libraryPath = pkgs.lib.makeLibraryPath [
            pkgs.gtk4-layer-shell
          ];
        in {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "yap";
            version = "0.0.12";
            src = self;

            cargoRoot = "Linux";
            buildAndTestSubdir = "Linux";
            cargoLock.lockFile = ./Linux/Cargo.lock;

            nativeBuildInputs = [
              pkgs.makeWrapper
              pkgs.pkg-config
              python
            ];

            buildInputs = [
              pkgs.glib
              pkgs.graphene
              pkgs.gtk3
              pkgs.gtk4
              pkgs.gtk4-layer-shell
              pkgs.libayatana-appindicator
              pkgs.pango
              pkgs.pipewire
            ];

            postInstall = ''
              install -Dm755 Linux/ui/yap-overlay "$out/bin/yap-overlay"
              install -Dm755 Linux/ui/yap-ui "$out/bin/yap-ui"
              install -Dm755 Linux/ui/yap-tray "$out/bin/yap-tray"

              for tool in yap-overlay yap-ui yap-tray; do
                substituteInPlace "$out/bin/$tool" \
                  --replace-fail "#!/usr/bin/python" "#!${python}/bin/python3"
              done

              for tool in yap yapctl yapd; do
                wrapProgram "$out/bin/$tool" \
                  --prefix PATH : "${runtimePath}"
              done

              for tool in yap-overlay yap-ui yap-tray; do
                wrapProgram "$out/bin/$tool" \
                  --prefix PATH : "${runtimePath}" \
                  --prefix LD_LIBRARY_PATH : "${libraryPath}" \
                  --set GI_TYPELIB_PATH "${typelibPath}" \
                  --set PYTHONPATH "${python}/${python.sitePackages}"
              done

              install -Dm644 Linux/packaging/dbus/com.yap.Yap.service \
                "$out/share/dbus-1/services/com.yap.Yap.service"
              install -Dm644 Linux/packaging/desktop/com.yap.Yap.Dashboard.desktop \
                "$out/share/applications/com.yap.Yap.Dashboard.desktop"
              install -Dm644 Linux/packaging/icons/com.yap.Yap.svg \
                "$out/share/icons/hicolor/scalable/apps/com.yap.Yap.svg"

              install -Dm644 Linux/packaging/systemd/yap.service \
                "$out/lib/systemd/user/yap.service"
              install -Dm644 Linux/packaging/systemd/yap-overlay.service \
                "$out/lib/systemd/user/yap-overlay.service"
              install -Dm644 Linux/packaging/systemd/yap-tray.service \
                "$out/lib/systemd/user/yap-tray.service"

              substituteInPlace "$out/lib/systemd/user/yap.service" \
                --replace-fail "/usr/bin/yapd" "$out/bin/yapd"
              substituteInPlace "$out/lib/systemd/user/yap-overlay.service" \
                --replace-fail "/usr/bin/yap-overlay" "$out/bin/yap-overlay"
              substituteInPlace "$out/lib/systemd/user/yap-tray.service" \
                --replace-fail "/usr/bin/yap-tray" "$out/bin/yap-tray"

              install -Dm644 README.md "$out/share/doc/yap/README.md"
              install -Dm644 LICENSE "$out/share/licenses/yap/LICENSE"
            '';

            meta = {
              description = "Local-first voice dictation for Wayland and Hyprland";
              homepage = "https://github.com/kylescudder/yap";
              license = pkgs.lib.licenses.mit;
              platforms = [ "x86_64-linux" ];
              mainProgram = "yap";
            };
          };
        });

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/yap";
        };
      });
    };
}
