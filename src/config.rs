use crate::storage::Bucket;
use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct Conns {
    pub bucket: Bucket,
    pub db: DatabaseConnection,
}
