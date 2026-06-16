{
  rs-harbor,
  pkgs,
  cross,
  toolchain,
  plinthProject,
}:
{
  devShells =
    (rs-harbor.lib.mkDevShells {
      inherit pkgs cross;
      inherit (toolchain) craneLib;
      pkgConfigDeps = [
        pkgs.udev
        pkgs.zstd
      ];
      packages = [
        pkgs.cargo-audit
        pkgs.cargo-deny
        pkgs.dosfstools
        pkgs.git
        pkgs.just
        pkgs.mtools
        pkgs.mdbook
        pkgs.qemu
        pkgs.ubootTools
        pkgs.util-linux
        pkgs.zola
        pkgs.zstd
      ];
      extraShellHook = ''
        echo "Website: cd website && zola serve"
        echo "Documentation: cd docs && mdbook serve"
      '';
    })
    // {
      docs = rs-harbor.lib.mkDocsShell {
        inherit pkgs cross;
        inherit (toolchain) craneLib;
        pkgConfigDeps = [
          pkgs.udev
          pkgs.zstd
        ];
        packages = [
          pkgs.mdbook
          plinthProject
        ];
        extraShellHook = ''
          echo "Project site: plinth-project serve --config website/plinth-project.toml"
          echo "Documentation: mdbook serve docs"
        '';
      };
    };
}
