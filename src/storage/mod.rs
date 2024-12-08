use object_store::{memory::InMemory, ObjectStore};

pub fn new_memory_bucket() -> impl ObjectStore {
    InMemory::new()
}
