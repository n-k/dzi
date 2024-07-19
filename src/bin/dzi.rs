use dzi::TileCreator;
use std::env;
use std::path::PathBuf;

pub fn main() {
    let args = env::args();
    if args.len() < 2 {
        eprintln!("Usage: dzi path/to/image");
        return;
    }
    let args: Vec<String> = args.into_iter().collect();
    let image_path = args.get(1).unwrap();
    let p = PathBuf::from(image_path.as_str());
    if !p.exists() {
        eprintln!("No such file {:?}", &p);
        return;
    }
    match TileCreator::new_from_image_path(p.as_path(), 254, 1) {
        Ok(ic) => match ic.create_tiles() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Could not tile image:\n\t {:?}", e);
            }
        },
        Err(e) => {
            eprintln!("Could not create tiler:\n\t {:?}", e);
        }
    }
}
