build: libcosim.a
libcosim.a: cosim.cpp
	g++ -std=c++20 -I ../third_party/riscv-isa-sim -O2 -c -g -o cosim.o cosim.cpp
	ar rcs libcosim.a cosim.o

clean:
	rm cosim.o || true
	rm libcosim.a || true
