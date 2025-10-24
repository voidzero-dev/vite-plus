use std::fs::File;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert!(args.len() == 3, "expected 2 arguments: <action> <file_path>");
    let action = args[1].as_str();
    let path = args[2].as_str();

    match action {
        "open_read" => {
            let _ = File::open(path);
        }
        "open_write" => {
            let _ = File::options().write(true).open(path);
        }
        "readdir" => {
            let _ = std::fs::read_dir(path);
        }
        _ => panic!("unknown action: {}", action),
    }
}
