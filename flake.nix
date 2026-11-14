{
  description = "Weft — federated GIF and animated media network";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f:
        nixpkgs.lib.genAttrs systems
          (system: f (import nixpkgs { inherit system; }) system);

      source = pkgs: pkgs.lib.cleanSourceWith {
        src = ./.;
        filter = path: _type:
          let base = baseNameOf (toString path);
          in !(builtins.elem base [ ".git" "target" "graphify-out" ".graphify" "data" ]);
      };
    in
    {
      devShells = forAllSystems (pkgs: _: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            python3
            postgresql_16
            curl
            jq
            pkg-config
          ];
          shellHook = ''
            echo "Weft dev shell"
            echo "  rustc: $(rustc --version)"
            echo "  postgres: ${pkgs.postgresql_16}/bin"
            echo "  docs: python3 tools/check_docs.py"
          '';
        };
      });

      packages = forAllSystems (pkgs: _:
        let
          weft = pkgs.rustPlatform.buildRustPackage {
            pname = "weft";
            version = "0.1.0";
            src = source pkgs;
            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildFlags = [ "--package" "weft-cli" "--package" "weft-validation" ];
            nativeBuildInputs = [ pkgs.pkg-config ];
            doCheck = false;
            installPhase = ''
              binary="$(find target -type f -name weft -path "*/release/*" | head -n 1)"
              test -n "$binary"
              install -Dm755 "$binary" $out/bin/weft
              decoder="$(find target -type f -name weft-decode -path "*/release/*" | head -n 1)"
              test -n "$decoder"
              install -Dm755 "$decoder" $out/bin/weft-decode
            '';
            meta = {
              description = "Weft node command line";
              license = pkgs.lib.licenses.agpl3Plus;
              mainProgram = "weft";
            };
          };
        in
        {
          inherit weft;
          default = weft;
        });

      apps = forAllSystems (_pkgs: system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.weft}/bin/weft";
          meta = {
            description = "Run a Weft node";
          };
        };
      });

      formatter = forAllSystems (pkgs: _: pkgs.rustfmt);
    };
}
