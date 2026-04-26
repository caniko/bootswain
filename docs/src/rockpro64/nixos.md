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
and adds a serial console matching the bootswain UART policy.

The lower-level `bootswain.nixosModules.rockpro64` module remains available
when a downstream flake wants to opt into those settings manually.

Keep hardware description and boot artifact integration separate when another
board-support module already owns the hardware stack. In that case, import the
hardware module alongside `bootswain.nixosModules.rockpro64` and set
`installExtlinux = false` when the host intentionally uses systemd-boot on an
ESP.
