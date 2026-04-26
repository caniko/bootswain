{self}: {
  config,
  lib,
  ...
}: {
  imports = [
    (import ./rockpro64.nix {inherit self;})
  ];

  config = {
    nixpkgs.hostPlatform = lib.mkDefault "aarch64-linux";

    boot.bootswain.rockpro64.enable = lib.mkDefault true;
    boot.loader.grub.enable = lib.mkDefault false;
    boot.loader.systemd-boot.enable = lib.mkDefault false;
    boot.loader.generic-extlinux-compatible.enable = lib.mkDefault true;

    boot.kernelParams = lib.mkAfter [
      "console=ttyS2,115200n8"
    ];
  };
}
