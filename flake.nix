{
  inputs.tooling.url = "github:mox-desktop/tooling";

  outputs =
    { tooling, ... }:
    tooling.lib.mkMoxFlake {
      devShells = tooling.lib.forAllSystems (pkgs: {
        default = pkgs.mkShell (
          pkgs.lib.fix (finalAttrs: {
            buildInputs = builtins.attrValues {
              inherit (pkgs)
                rustToolchain
                rust-analyzer-unwrapped
                nixd
                vulkan-loader
                vulkan-headers
                vulkan-validation-layers
                wgsl-analyzer
                pkg-config
                libxkbcommon
                libGL
                wayland
                ;
              inherit (pkgs.xorg)
                libXcursor
                libXrandr
                libXi
                libX11
                ;
            };
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath finalAttrs.buildInputs;
            RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
          })
        );
      });
    };
}
