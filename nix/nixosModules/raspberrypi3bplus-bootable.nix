{self}: {
  lib,
  ...
}: {
  imports = [
    (import ./raspberrypi3bplus.nix {inherit self;})
  ];

  config = {
    nixpkgs.hostPlatform = lib.mkDefault "aarch64-linux";

    boot.bootswain.raspberryPi3BPlus.enable = lib.mkDefault true;
    boot.loader.grub.enable = lib.mkDefault false;
    boot.loader.systemd-boot.enable = lib.mkDefault false;
    boot.loader.generic-extlinux-compatible.enable = lib.mkDefault true;

    boot.kernelParams = lib.mkAfter [
      "console=ttyAMA0,115200n8"
      "console=tty0"
    ];
  };
}
