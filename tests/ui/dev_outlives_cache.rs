// Dev must not outlive Cache.
// This should fail to compile because the Dev borrows the Cache.

use blkid::cache::Cache;
use blkid::dev::GetDevFlags;

fn main() {
    let dev = {
        let cache = Cache::new().unwrap();
        cache.probe_all().unwrap();
        cache.get_dev("/dev/null", GetDevFlags::FIND).unwrap()
    };
    // cache is dropped here, but dev still references it — must not compile
    let _ = dev.name();
}
