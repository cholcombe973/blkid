// Devs iterator must not outlive Cache.
// This should fail to compile because the Devs borrows the Cache.

use blkid::cache::Cache;

fn main() {
    let devs = {
        let cache = Cache::new().unwrap();
        cache.probe_all().unwrap();
        cache.devs()
    };
    // cache is dropped here, but devs still references it — must not compile
    for dev in devs {
        let _ = dev.name();
    }
}
