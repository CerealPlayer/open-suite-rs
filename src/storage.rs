use s3::creds::Credentials;
use s3::error::S3Error;
use s3::{BucketConfiguration, Region};

pub type Bucket = Box<s3::Bucket>;

pub async fn get_bucket(bucket_name: &str, region: Region) -> Result<Bucket, S3Error> {
    let credentials = Credentials::default()?;

    let mut bucket =
        s3::Bucket::new(bucket_name, region.clone(), credentials.clone())?.with_path_style();

    if !bucket.exists().await? {
        bucket = s3::Bucket::create_with_path_style(
            bucket_name,
            region,
            credentials,
            BucketConfiguration::default(),
        )
        .await?
        .bucket
    }

    Ok(bucket)
}

pub async fn upload_bytes(bucket: &Bucket, path: &str, bytes: &[u8]) -> Result<(), S3Error> {
    bucket.put_object(path, bytes).await?;
    Ok(())
}
