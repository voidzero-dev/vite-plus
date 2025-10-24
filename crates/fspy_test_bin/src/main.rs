fn main() {
    eprintln!("zz");
    let _ = std::fs::File::open("hello");

    eprintln!("bb");
}
