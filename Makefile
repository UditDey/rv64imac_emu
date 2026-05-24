.PHONY: build dtb libcosim buildroot spike
.PHONY: clean clean_dtb clean_libcosim clean_buildroot clean_spike

build: dtb buildroot
clean: clean_dtb clean_libcosim clean_buildroot clean_spike

dtb: device_tree.dtb
device_tree.dtb: device_tree.dts
	dtc -I dts -O dtb -o $@ $<
clean_dtb:
	rm device_tree.dtb || true

libcosim: spike
	$(MAKE) -C libcosim
clean_libcosim:
	$(MAKE) -C libcosim clean

buildroot:
	cp buildroot.cfg third_party/buildroot/.config
	export PATH="/bin" && $(MAKE) -C third_party/buildroot BR2_JLEVEL=3
clean_buildroot:
	$(MAKE) -C third_party/buildroot clean

spike:
	if [ ! -e third_party/riscv-isa-sim/build ]; then \
		mkdir third_party/riscv-isa-sim/build; \
		cd third_party/riscv-isa-sim/build && ../configure; \
	fi
	$(MAKE) -C third_party/riscv-isa-sim/build -j10

clean_spike:
	rm -r third_party/riscv-isa-sim/build || true
