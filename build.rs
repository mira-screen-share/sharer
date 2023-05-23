extern crate embed_resource;

fn main() {
    if cfg!(target_os = "windows") {
        embed_resource::compile("mira-manifest.rc", embed_resource::NONE);
    }
}
