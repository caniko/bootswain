{ rs-harbor, pkgs, cross, toolchain }:
{
  devShells = rs-harbor.lib.mkDevShells {
    inherit pkgs cross;
    inherit (toolchain) craneLib;
    pkgConfigDeps = [
      pkgs.udev
      pkgs.zstd
    ];
    packages = [
      pkgs.dosfstools
      pkgs.git
      pkgs.just
      pkgs.mtools
      pkgs.qemu
      pkgs.ubootTools
      pkgs.util-linux
      pkgs.zstd
    ];
  };
}
