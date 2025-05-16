fn main() {
    let mut cc = capnpc::CompilerCommand::new();
    let mut cc = cc.src_prefix("schema");
    for entry in std::fs::read_dir("schema").expect("read schema dir") {
        if let Ok(e) = entry {
            if e.file_name()
                .into_string()
                .expect("conversion to string of filename")
                .ends_with(".capnp")
                && e.file_type().expect("checking file type").is_file()
            {
                if let Some(s) = e.file_name().to_str() {
                    cc = cc.file("schema/".to_owned() + s)
                }
            }
        }
    }
    cc.run().expect("schema compiler command");
}
