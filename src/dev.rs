// Updated Dev structure in src/dev.rs

pub struct Device<'a> {
    // existing fields...
    // Add any necessary adjustments here
    phantom: std::marker::PhantomData<&'a ()>,
}

// Additional necessary adjustments can be made here.