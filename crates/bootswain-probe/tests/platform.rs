use bootswain_core::Board;
use bootswain_probe::probe_platform;

#[test]
fn platform_probe_binds_the_dt_board_and_does_not_infer_runtime_qualification() {
    let root = tempfile::tempdir().unwrap();
    let sys = root.path().join("sys");
    let proc = root.path().join("proc");
    std::fs::create_dir_all(sys.join("firmware/devicetree/base")).unwrap();
    std::fs::create_dir_all(sys.join("firmware/efi/efivars")).unwrap();
    std::fs::create_dir_all(proc.join("sys/kernel")).unwrap();
    std::fs::write(
        sys.join("firmware/devicetree/base/compatible"),
        b"radxa,rock-5a\0rockchip,rk3588s\0",
    )
    .unwrap();
    std::fs::write(proc.join("sys/kernel/osrelease"), "7.2.8\n").unwrap();
    let report = probe_platform(&sys, &proc, Board::Rock5A).unwrap();
    assert_eq!(report.board, Board::Rock5A);
    assert!(report.efi_booted && report.efivars_visible);
    assert_eq!(report.runtime_variable_write, "not-measured");
    assert_eq!(report.watchdog_reset, "not-measured");
    assert!(probe_platform(&sys, &proc, Board::RaspberryPi5).is_err());
    std::fs::write(
        sys.join("firmware/devicetree/base/compatible"),
        b"raspberrypi,5-model-b\0brcm,bcm2712\0",
    )
    .unwrap();
    assert_eq!(
        probe_platform(&sys, &proc, Board::RaspberryPi5)
            .unwrap()
            .board,
        Board::RaspberryPi5
    );
    std::fs::write(
        sys.join("firmware/devicetree/base/compatible"),
        b"raspberrypi,compute-module-5\0brcm,bcm2712\0",
    )
    .unwrap();
    assert!(probe_platform(&sys, &proc, Board::RaspberryPi5).is_err());
}
