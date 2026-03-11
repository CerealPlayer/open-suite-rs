use crate::storage::Bucket;

pub struct Conns {
    pub bucket: Bucket,
}

impl Clone for Conns {
    fn clone(&self) -> Self {
        Self {
            bucket: self.bucket.clone(),
        }
    }
}
