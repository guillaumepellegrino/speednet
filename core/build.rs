
fn main() {
    cc::Build::new()
        .file("src/bpf.c")
        .compile("bpf");
}
