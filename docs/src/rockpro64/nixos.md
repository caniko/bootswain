# NixOS Integration

Downstream flakes can import the bootable ROCKPro64 profile and layer their own
filesystems, users, services, and deployment policy on top:

```nix
{
  inputs.bootswain.url = "github:caniko/bootswain";

  outputs = {
    nixpkgs,
    bootswain,
    ...
  }: {
    nixosConfigurations.rockpro64 = nixpkgs.lib.nixosSystem {
      system = "aarch64-linux";
      modules = [
        bootswain.nixosModules.rockpro64Bootable
        ./configuration.nix
      ];
    };
  };
}
```

The profile enables the bootswain ROCKPro64 integration, disables GRUB, enables
NixOS generic extlinux output, sets `nixpkgs.hostPlatform` to `aarch64-linux`,
and adds a serial console matching the bootswain UART policy. It is intended for
shared bootable images where extlinux is the compatibility path.

The lower-level `bootswain.nixosModules.rockpro64` module remains available
when a downstream flake wants to opt into those settings manually. Its default
operating-system handoff is EFI/systemd-boot: it prepares
`/EFI/BOOT/BOOTAA64.EFI` on the ESP and keeps EFI variable writes disabled by
default.

Keep hardware description and boot artifact integration separate when another
board-support module already owns the hardware stack. In that case, import the
hardware module alongside `bootswain.nixosModules.rockpro64`. Set
`boot.bootswain.rockpro64.osBootProtocol = "extlinux"` only when the host should
use NixOS' generic extlinux-compatible bootloader output.
