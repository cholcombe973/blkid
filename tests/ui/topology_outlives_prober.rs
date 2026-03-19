// Topology must not outlive Prober.
// This should fail to compile because the Topology borrows the Prober.

use blkid::prober::Prober;

fn main() {
    let topo = {
        let prober = Prober::new().unwrap();
        prober.topology().unwrap()
    };
    // prober is dropped here, but topo still references it — must not compile
    let _ = topo.alignment_offset();
}
