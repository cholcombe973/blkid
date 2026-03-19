// PartList must not outlive Prober.
// This should fail to compile because the PartList borrows the Prober.

use blkid::prober::Prober;

fn main() {
    let part_list = {
        let prober = Prober::new().unwrap();
        prober.part_list().unwrap()
    };
    // prober is dropped here, but part_list still references it — must not compile
    let _ = part_list.numof_partitions();
}
