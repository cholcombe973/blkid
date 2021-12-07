const LIB_NAME: &str = "blkid";
const BLKID_MIN_REQ_VERSION: &str = "2.21.0";
/// MIN numbers of versions where were added new functionality
const BLKID_CHANGED_MIN_VERSIONS: &[usize] = &[23, 24, 25, 30, 31, 36, 37];

fn main() {
    let libblkid = pkg_config::Config::new()
        .atleast_version(BLKID_MIN_REQ_VERSION)
        .probe(LIB_NAME)
        .expect("Failed to find minimal required version of library");

    // Take a MIN version from: `MAJ.MIN.PATCH`
    let min_num = libblkid
        .version
        .split_terminator('.')
        .nth(1)
        .expect("Failed to find MIN number of version");
    // Parse version to figure out what features should to be enabled
    let min_num: usize = min_num
        .parse()
        .expect("Failed to parse MIN number of version");

    // Find the index of the last changed version
    let idx = BLKID_CHANGED_MIN_VERSIONS.iter().position(|v| v > &min_num);

    if let Some(idx) = idx {
        // If we have some index this means not all features need to be enabled
        for min_num in &BLKID_CHANGED_MIN_VERSIONS[..idx] {
            println!("cargo:rustc-cfg={}=\"2.{}\"", LIB_NAME, min_num);
        }
    } else {
        // In this case we just enable all features
        for min_num in BLKID_CHANGED_MIN_VERSIONS {
            println!("cargo:rustc-cfg={}=\"2.{}\"", LIB_NAME, min_num);
        }
    }
}
