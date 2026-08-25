// POC for iterating on send_print::upload_via_ftps against a real printer without going through
// the axum app / auth / UI each time.
//
//   cargo run --example ftps_poc -- <ip> <access_code> <local_file> <remote_file_name>
use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, ip, access_code, local_file, remote_name] = args.as_slice() else {
        eprintln!("usage: ftps_poc <ip> <access_code> <local_file> <remote_file_name>");
        std::process::exit(1);
    };

    let mut file = std::fs::File::open(local_file).expect("open local_file");
    let file_len = file.metadata().expect("metadata").len();

    let mut easy = curl::easy::Easy::new();
    easy.url(&format!("ftps://{ip}:990/{remote_name}")).unwrap();
    easy.username("bblp").unwrap();
    easy.password(access_code).unwrap();
    easy.ssl_verify_peer(false).unwrap();
    easy.ssl_verify_host(false).unwrap();
    easy.upload(true).unwrap();
    easy.in_filesize(file_len).unwrap();
    easy.verbose(true).unwrap();
    easy.debug_function(|kind, data| {
        let prefix = match kind {
            curl::easy::InfoType::HeaderIn => "< ",
            curl::easy::InfoType::HeaderOut => "> ",
            curl::easy::InfoType::Text => "* ",
            _ => return,
        };
        eprint!("{prefix}{}", String::from_utf8_lossy(data));
    })
    .unwrap();

    let mut transfer = easy.transfer();
    transfer.read_function(move |buf| Ok(file.read(buf).unwrap_or(0))).unwrap();
    match transfer.perform() {
        Ok(()) => println!("upload OK"),
        Err(e) => {
            eprintln!("upload FAILED: {e}");
            std::process::exit(1);
        }
    }
}
