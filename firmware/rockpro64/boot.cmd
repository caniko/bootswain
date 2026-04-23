# bootswain ROCKPro64 boot script scaffold.
#
# This is a source fragment, not a validated boot.scr binary.

echo "bootswain ROCKPro64 firmware scaffold"
echo "board=${bootswain_board}"
echo "soc=${bootswain_soc}"
echo "validation=${bootswain_validation}"
echo "targets=${boot_targets}"

if test "${bootswain_validation}" != "performed"; then
	echo "Hardware validation has not been performed in this tree."
fi

setenv bootswain_menu_entry_1 "Boot installed system"
setenv bootswain_menu_entry_2 "Flash SPI firmware"
setenv bootswain_menu_entry_3 "Erase SPI firmware"
setenv bootswain_menu_entry_4 "Enter U-Boot shell"
echo "Menu actions are declared as scaffolding only."
