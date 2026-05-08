fn main() {
    println!("cargo:rerun-if-changed=../libcosim/libcosim.a");
    println!("cargo:rerun-if-changed=../third_party/riscv-isa-sim/build/libriscv.a");

    println!("cargo:rustc-link-search=native=../libcosim");
    println!("cargo:rustc-link-search=native=../third_party/riscv-isa-sim/build");

    println!("cargo:rustc-link-lib=static=cosim");
    println!("cargo:rustc-link-lib=static=riscv");
    println!("cargo:rustc-link-lib=static=fesvr");
    println!("cargo:rustc-link-lib=static=disasm");
    println!("cargo:rustc-link-lib=static=softfloat");
    println!("cargo:rustc-link-lib=dylib=stdc++");
}
