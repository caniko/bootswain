# Armbian's vendor-tuned rockchip64 kernel, consumed prebuilt instead of
# compiling a generic aarch64 kernel. The debs are pinned by the SHA-256
# from Armbian's signed apt index (noble InRelease, signing key
# 8CFA83D13EB2181EEF5843E41EB30FAF236099FE). `./config` is the image deb's
# /boot/config-*, vendored so the kernel config is readable without
# import-from-derivation; the build fails if the two ever diverge.
#
# ponytail: apt.armbian.com prunes old pool versions; once fetched, the
# unpacked output lives in the binary cache. Bump all hashes together.
{
  lib,
  stdenv,
  stdenvNoCC,
  fetchurl,
  dpkg,
  ...
} @ args:
# NixOS always re-calls this through kernel.override to merge kernelPatches,
# features and randstructSeed. A prebuilt binary cannot honour patches, so
# refuse them instead of silently dropping them. Not a formal: callPackage
# would auto-fill it with pkgs.kernelPatches.
assert lib.assertMsg ((args.kernelPatches or []) == []) "armbian-rockchip64-kernel is prebuilt and cannot apply boot.kernelPatches"; let
  version = "6.18.44";
  modDirVersion = "${version}-current-rockchip64";
  debSuffix = "_26.8.3_arm64__${version}-S1efe-D5397-Pd9e5-C40fc-H2153-HK01ba-Vc222-B4990-R448a.deb";
  image = fetchurl {
    url = "https://apt.armbian.com/pool/main/l/linux-${version}/linux-image-current-rockchip64${debSuffix}";
    hash = "sha256-iHOHiHAtfvPTShgGOfBBfgouavxmnGgefeNhtJ2M6ro=";
  };
  dtbs = fetchurl {
    url = "https://apt.armbian.com/pool/main/l/linux-dtb-current-rockchip64/linux-dtb-current-rockchip64${debSuffix}";
    hash = "sha256-kH6pCtaOyzIafJ8a8Ick0b1dEgYTmPENgKDh5cVfdq4=";
  };
  configfile = ./config;

  # Same shape as nixpkgs' kernel `config` passthru (pkgs/os-specific/linux/kernel/build.nix).
  config = let
    values = lib.listToAttrs (lib.concatMap (line: let
      m = lib.match "(CONFIG_[^=]+)=([ym])" line;
    in
      lib.optional (m != null) {
        name = lib.elemAt m 0;
        value = lib.elemAt m 1;
      }) (lib.splitString "\n" (builtins.readFile configfile)));
    get = attr: values."CONFIG_${attr}" or null;
  in
    values
    // rec {
      isSet = attr: get attr != null;
      getValue = get;
      isYes = attr: get attr == "y";
      isNo = attr: get attr == "n";
      isModule = attr: get attr == "m";
      isEnabled = attr: isYes attr || isModule attr;
      isDisabled = attr: !(isEnabled attr);
    };
in
  stdenvNoCC.mkDerivation {
    pname = "linux-armbian-rockchip64";
    inherit version;
    nativeBuildInputs = [dpkg];
    buildCommand = ''
      dpkg-deb -x ${image} image
      dpkg-deb -x ${dtbs} dtbs
      cmp image/boot/config-${modDirVersion} ${configfile}
      mkdir -p $out/lib
      install -m 0644 image/boot/vmlinuz-${modDirVersion} $out/Image
      install -m 0644 image/boot/System.map-${modDirVersion} $out/System.map
      install -m 0644 ${configfile} $out/config
      mv image/lib/modules $out/lib/modules
      mv dtbs/boot/dtb-${modDirVersion} $out/dtbs
    '';
    passthru = {
      inherit version modDirVersion config configfile stdenv;
      target = "Image";
      buildDTBs = true; # ships dtbs/, keeping hardware.deviceTree enabled
      isZen = false;
      # NixOS skips system.requiredKernelConfig for kernels exposing
      # `features`; the consuming host re-asserts it against `config`.
      features = lib.optionalAttrs (config.isYes "EFI_STUB") {efiBootStub = true;};
      kernelOlder = lib.versionOlder version;
      kernelAtLeast = lib.versionAtLeast version;
    };
    meta = {
      description = "Armbian prebuilt rockchip64 Linux kernel";
      license = lib.licenses.gpl2Only;
      platforms = ["aarch64-linux"];
      sourceProvenance = [lib.sourceTypes.binaryNativeCode];
    };
  }
